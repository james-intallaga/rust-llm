import SwiftUI
import UIKit
import ForgeSwift

@main
struct AmmaRecognizeApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup {
            MainView()
        }
    }
}

// MARK: - App Delegate for Early Keyboard Preloading
class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil) -> Bool {
        // Initialize the Forge backend early
        initializeForge()

        // Preload keyboard as early as possible - this is what ChatGPT/Apollo do
        KeyboardPreloader.shared.preloadImmediately()
        return true
    }

    func applicationWillTerminate(_ application: UIApplication) {
        // Cleanup the Forge backend when app terminates
        cleanupForge()
    }
}

// MARK: - Keyboard Preloader
/// Preloads the iOS keyboard during app startup to eliminate the ~1 second delay
/// that occurs when the keyboard is first invoked. This is the industry-standard
/// technique used by ChatGPT, Apollo, and other high-quality chat apps.
final class KeyboardPreloader {
    static let shared = KeyboardPreloader()
    private var preloadWindow: UIWindow?
    private var hiddenTextField: UITextField?

    private init() {}

    /// Called from AppDelegate - preloads keyboard resources synchronously
    func preloadImmediately() {
        // Create a temporary window and text field for keyboard preloading
        // This must happen on main thread and as early as possible
        DispatchQueue.main.async { [weak self] in
            self?.performPreload()
        }
    }

    private func performPreload() {
        // Get the first window scene
        guard let windowScene = UIApplication.shared.connectedScenes
            .compactMap({ $0 as? UIWindowScene })
            .first else {
            // Retry after a short delay if scene isn't ready yet
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
                self?.performPreload()
            }
            return
        }

        // Create a temporary window at a very low level
        let tempWindow = UIWindow(windowScene: windowScene)
        tempWindow.windowLevel = UIWindow.Level(rawValue: -1000)
        tempWindow.frame = CGRect(x: 0, y: 0, width: 1, height: 1)
        tempWindow.alpha = 0.01 // Nearly invisible

        let viewController = UIViewController()
        tempWindow.rootViewController = viewController
        tempWindow.makeKeyAndVisible()

        // Create text field with all keyboard features to fully preload
        let textField = UITextField(frame: .zero)
        textField.autocorrectionType = .default
        textField.autocapitalizationType = .sentences
        textField.spellCheckingType = .default
        textField.returnKeyType = .send
        viewController.view.addSubview(textField)

        // Store references
        self.preloadWindow = tempWindow
        self.hiddenTextField = textField

        // Trigger keyboard load
        textField.becomeFirstResponder()

        // Keep keyboard loaded briefly, then clean up
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in
            self?.cleanup()
        }
    }

    private func cleanup() {
        hiddenTextField?.resignFirstResponder()
        hiddenTextField?.removeFromSuperview()
        hiddenTextField = nil

        preloadWindow?.isHidden = true
        preloadWindow?.rootViewController = nil
        preloadWindow = nil
    }
}
