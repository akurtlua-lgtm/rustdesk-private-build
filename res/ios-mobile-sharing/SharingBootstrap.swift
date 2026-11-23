import Foundation
import Security

struct SharingStatus: Codable {
    let nonce: String
    let phase: Int32
    let error: Int32
    let deviceID: String
}

struct SharingBootstrap: Codable {
    let nonce: String
    let servers: Data
    let policy: Data
    let password: String

    enum Failure: Error { case configuration, keychain, container, expired }
    static let service = "RustDesk.Private.Broadcast.Once"

    static func setting(_ name: String) throws -> String {
        guard let value = Bundle.main.object(forInfoDictionaryKey: name) as? String,
              !value.isEmpty, !value.contains("$(") else { throw Failure.configuration }
        return value
    }
    static func groupURL() throws -> URL {
        guard let url = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: try setting("RustDeskSharingAppGroup")) else {
            throw Failure.container
        }
        return url
    }
    static func snapshot<Size: BinaryInteger>(_ getter: (UnsafeMutablePointer<UInt8>?, Int) -> Size) throws -> Data {
        guard let size = Int(exactly: getter(nil, 0)), size > 1, size <= 16384 else {
            throw Failure.configuration
        }
        var bytes = [UInt8](repeating: 0, count: size)
        defer { _ = bytes.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) } }
        let written = bytes.withUnsafeMutableBufferPointer { getter($0.baseAddress, $0.count) }
        guard Int(exactly: written) == size, bytes[size - 1] == 0 else { throw Failure.configuration }
        return Data(bytes.dropLast())
    }
    static func query() throws -> [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: service,
         kSecAttrAccount as String: "next-authorized-broadcast",
         kSecAttrAccessGroup as String: try setting("RustDeskSharingKeychainGroup"),
         kSecAttrSynchronizable as String: false]
    }
    static func prepare(password: String) throws -> SharingBootstrap {
        _ = try groupURL()
        let bootstrap = SharingBootstrap(nonce: UUID().uuidString,
            servers: try snapshot(rd_mobile_host_servers),
            policy: try snapshot(rd_mobile_host_legacy_policy), password: password)
        var attributes = try query()
        let removed = SecItemDelete(attributes as CFDictionary)
        guard removed == errSecSuccess || removed == errSecItemNotFound else { throw Failure.keychain }
        attributes[kSecValueData as String] = try JSONEncoder().encode(bootstrap)
        attributes[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        guard SecItemAdd(attributes as CFDictionary, nil) == errSecSuccess else { throw Failure.keychain }
        try writeStatus(SharingStatus(nonce: bootstrap.nonce, phase: 1, error: 0, deviceID: ""))
        return bootstrap
    }
    static func consume() throws -> SharingBootstrap {
        var attributes = try query()
        attributes[kSecReturnData as String] = true
        attributes[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(attributes as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data, data.count <= 65536 else { throw Failure.keychain }
        guard SecItemDelete(try query() as CFDictionary) == errSecSuccess else { throw Failure.keychain }
        let bootstrap = try JSONDecoder().decode(SharingBootstrap.self, from: data)
        guard try status()?.nonce == bootstrap.nonce else { throw Failure.expired }
        return bootstrap
    }
    static func discard() throws {
        let result = SecItemDelete(try query() as CFDictionary)
        guard result == errSecSuccess || result == errSecItemNotFound else { throw Failure.keychain }
    }
    static func writeStatus(_ status: SharingStatus) throws {
        try JSONEncoder().encode(status).write(to: groupURL().appendingPathComponent("broadcast-status.json"),
                                               options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
    }
    static func status() throws -> SharingStatus? {
        let url = try groupURL().appendingPathComponent("broadcast-status.json")
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        return try JSONDecoder().decode(SharingStatus.self, from: Data(contentsOf: url))
    }
    static func requestStop(_ nonce: String) throws {
        try Data(nonce.utf8).write(to: groupURL().appendingPathComponent("broadcast-stop"), options: .atomic)
    }
    static func stopRequested(_ nonce: String) throws -> Bool {
        let url = try groupURL().appendingPathComponent("broadcast-stop")
        guard FileManager.default.fileExists(atPath: url.path) else { return false }
        return try Data(contentsOf: url) == Data(nonce.utf8)
    }
    func configureExtension() throws {
        guard let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            throw Failure.container
        }
        let directory = support.appendingPathComponent("RustDeskShare", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
                                               attributes: [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication])
        let bytes = Array(directory.path.utf8)
        let initialized = bytes.withUnsafeBufferPointer { rd_mobile_host_initialize($0.baseAddress, $0.count) }
        guard initialized == 0 else { throw Failure.configuration }
        let configured = servers.withUnsafeBytes { rd_mobile_host_configure_servers($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard configured == 0 else { throw Failure.configuration }
        let pass = Array(password.utf8)
        let protected = pass.withUnsafeBufferPointer { rd_mobile_host_set_password($0.baseAddress, $0.count) }
        guard protected == 0 else { throw Failure.configuration }
        let imported = policy.withUnsafeBytes { rd_mobile_host_legacy_import($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard imported == 0 else { throw Failure.configuration }
    }
}
