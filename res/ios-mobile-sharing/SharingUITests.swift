import XCTest

final class SharingUITests: XCTestCase {
    func testSimulatorOffersSharingInterfaceWithoutClaimingSystemCapture() {
        let app = XCUIApplication()
        app.launchArguments += ["-AppleLanguages", "(en)", "-AppleLocale", "en_US"]
        app.launch()
        let english = app.buttons["Share screen"]
        let chinese = app.buttons["共享屏幕"]
        XCTAssertTrue(english.waitForExistence(timeout: 30) || chinese.exists)
        (english.exists ? english : chinese).tap()
        let status = app.staticTexts["private-sharing-status"]
        XCTAssertTrue(status.waitForExistence(timeout: 10))
        XCTAssertTrue(status.label.contains("模拟器不支持系统屏幕共享"))
        XCTAssertFalse(app.buttons["private-sharing-start"].isEnabled)
        app.buttons["关闭"].tap()
        XCTAssertTrue(english.waitForExistence(timeout: 10) || chinese.exists)
    }
}
