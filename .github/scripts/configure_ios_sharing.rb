# Apply the development sharing variant only inside an isolated build checkout.
require 'xcodeproj'
require 'pathname'

root = Pathname.new(__dir__).join('../..').expand_path
id = ENV.fetch('MOBILE_APPLICATION_ID')
raise 'Malformed application identity' unless id.match?(/\A[a-zA-Z][a-zA-Z0-9]*(?:\.[a-zA-Z0-9]+)+\z/)
project_path = root.join('flutter/ios/Runner.xcodeproj')
project = Xcodeproj::Project.open(project_path)
runner = project.targets.find { |target| target.name == 'Runner' }
raise 'Missing Runner target' unless runner
raise 'Sharing variant already configured' if project.targets.any? { |target| target.name == 'BroadcastShare' }
common = %w[res/ios-capture-frame/CaptureFrame.swift res/ios-capture-frame/CaptureDelivery.swift
            res/ios-mobile-sharing/NativeSharingHost.swift res/ios-mobile-sharing/SharingBootstrap.swift]
app = common + %w[res/ios-mobile-sharing/SharingViewController.swift res/ios-mobile-sharing/SharingPlugin.swift]
sdk = `xcrun --sdk iphoneos --show-sdk-version`.strip
raise 'Cannot determine SDK' unless $?.success? && sdk.match?(/\A\d+(?:\.\d+)+\z/)
app += %w[res/ios27-capture/VideoCaptureAPI.swift res/ios27-capture/SharingCapture.swift] if sdk.split('.').first.to_i >= 27
extension = project.new_target(:app_extension, 'BroadcastShare', :ios, '15.0')
shared_refs = {}
(app + common + %w[res/ios-capture-frame/BroadcastCaptureHandler.swift res/ios-mobile-sharing/SampleHandler.swift]).uniq.each do |file|
  shared_refs[file] = project.main_group.new_file(root.join(file).relative_path_from(project_path.dirname).to_s)
end
app.each { |file| runner.source_build_phase.add_file_reference(shared_refs.fetch(file)) }
(common + %w[res/ios-capture-frame/BroadcastCaptureHandler.swift res/ios-mobile-sharing/SampleHandler.swift]).each do |file|
  extension.source_build_phase.add_file_reference(shared_refs.fetch(file))
end
entitlements = {
  'com.apple.security.application-groups' => ["group.#{id}.sharing"],
  'keychain-access-groups' => ["$(AppIdentifierPrefix)#{id}.sharing"]
}
Xcodeproj::Plist.write_to_path(entitlements, root.join('flutter/ios/Sharing.entitlements'))
runner_entitlements = Xcodeproj::Plist.read_from_path(root.join('flutter/ios/Runner/Runner.entitlements'))
Xcodeproj::Plist.write_to_path(runner_entitlements.merge(entitlements), root.join('flutter/ios/Runner/Sharing.entitlements'))
info = Xcodeproj::Plist.read_from_path(root.join('res/ios-host-probe/Info.plist'))
info['NSExtension']['NSExtensionPrincipalClass'] = '$(PRODUCT_MODULE_NAME).SampleHandler'
info['RustDeskSharingAppGroup'] = "group.#{id}.sharing"
info['RustDeskSharingKeychainGroup'] = "$(AppIdentifierPrefix)#{id}.sharing"
Xcodeproj::Plist.write_to_path(info, root.join('flutter/ios/BroadcastShare-Info.plist'))
runner_info = Xcodeproj::Plist.read_from_path(root.join('flutter/ios/Runner/Info.plist'))
runner_info['RustDeskSharingAppGroup'] = info['RustDeskSharingAppGroup']
runner_info['RustDeskSharingKeychainGroup'] = info['RustDeskSharingKeychainGroup']
runner_info['UIBackgroundModes'] = ((runner_info['UIBackgroundModes'] || []) + ['screen-capture']).uniq if sdk.split('.').first.to_i >= 27
Xcodeproj::Plist.write_to_path(runner_info, root.join('flutter/ios/Runner/Info.plist'))
runner.build_configurations.each do |config|
  config.build_settings['SWIFT_ACTIVE_COMPILATION_CONDITIONS'] = (Array(config.build_settings['SWIFT_ACTIVE_COMPILATION_CONDITIONS'] || '$(inherited)') + ['RD_PRIVATE_MOBILE_SCREEN_SHARING']).uniq
  config.build_settings['GCC_PREPROCESSOR_DEFINITIONS'] = (Array(config.build_settings['GCC_PREPROCESSOR_DEFINITIONS'] || '$(inherited)') + ['RD_PRIVATE_MOBILE_SCREEN_SHARING=1']).uniq
  config.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'Runner/Sharing.entitlements'
end
extension.build_configurations.each do |config|
  config.build_settings.merge!({
    'PRODUCT_BUNDLE_IDENTIFIER' => "#{id}.BroadcastShare", 'PRODUCT_NAME' => 'BroadcastShare',
    'INFOPLIST_FILE' => 'BroadcastShare-Info.plist', 'CODE_SIGN_ENTITLEMENTS' => 'Sharing.entitlements',
    'SWIFT_OBJC_BRIDGING_HEADER' => '$(SRCROOT)/../../res/mobile-sharing-core/host.h',
    'SWIFT_VERSION' => '5.0', 'APPLICATION_EXTENSION_API_ONLY' => 'YES',
    'MARKETING_VERSION' => ENV.fetch('VERSION'), 'CURRENT_PROJECT_VERSION' => ENV.fetch('BUILD_NUMBER'),
    'SKIP_INSTALL' => 'YES', 'STRIP_INSTALLED_PRODUCT' => 'NO',
    'RUSTDESK_IOS_CORE_LIBRARY' => '$(PROJECT_DIR)/../../target/aarch64-apple-ios/release/liblibrustdesk.a',
    'RUSTDESK_IOS_CORE_LIBRARY[sdk=iphonesimulator*]' => '$(PROJECT_DIR)/../../target/aarch64-apple-ios-sim/release/liblibrustdesk.a',
    'OTHER_LDFLAGS' => ['$(inherited)', '$(RUSTDESK_IOS_CORE_LIBRARY)', '-Wl,-dead_strip', '-lc++', '-lsqlite3',
      *%w[Security SystemConfiguration AVFoundation AudioToolbox CoreAudio CoreMedia CoreVideo CoreGraphics CoreImage VideoToolbox].flat_map { |name| ['-framework', name] }]
  })
end
runner.add_dependency(extension)
embed = runner.new_copy_files_build_phase('Embed Broadcast Extension')
embed.dst_subfolder_spec = '13'
embed.add_file_reference(extension.product_reference).settings = { 'ATTRIBUTES' => ['RemoveHeadersOnCopy'] }
ui = project.new_target(:ui_test_bundle, 'SharingUITests', :ios, '15.0')
ui.source_build_phase.add_file_reference(project.main_group.new_file('../../res/ios-mobile-sharing/SharingUITests.swift'))
ui.add_dependency(runner)
ui.build_configurations.each do |config|
  config.build_settings.merge!({ 'PRODUCT_BUNDLE_IDENTIFIER' => id + '.SharingUITests',
    'TEST_TARGET_NAME' => 'Runner', 'SWIFT_VERSION' => '5.0', 'GENERATE_INFOPLIST_FILE' => 'YES' })
end
scheme = Xcodeproj::XCScheme.new
scheme.add_build_target(runner)
scheme.add_test_target(ui)
scheme.set_launch_target(runner)
scheme.save_as(project_path, 'PrivateSharing', true)
project.save
puts 'Configured isolated iOS sharing variant; no signing credentials included.'
