// AppDelegate.swift
// OISPApp
//
// Menu bar app delegate - manages NSStatusItem and popover

import SwiftUI
import AppKit

class AppDelegate: NSObject, NSApplicationDelegate {
    // MARK: - Properties

    private var statusItem: NSStatusItem?
    private var popover: NSPopover?
    private var eventMonitor: Any?

    // MARK: - Lifecycle

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Hide dock icon (menu bar only app)
        NSApp.setActivationPolicy(.accessory)

        // Create status item
        setupStatusItem()

        // Create popover
        setupPopover()

        // Monitor for clicks outside popover
        setupEventMonitor()

        // Check first launch
        checkFirstLaunch()
    }

    func applicationWillTerminate(_ notification: Notification) {
        // Cleanup
    }

    // MARK: - Status Item

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem?.button {
            // Use SF Symbol for menu bar icon
            button.image = NSImage(systemSymbolName: "brain", accessibilityDescription: "OISP")
            button.action = #selector(togglePopover)
            button.target = self

            // Make button respond to right-click too
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }
    }

    // MARK: - Popover

    private func setupPopover() {
        let popover = NSPopover()
        popover.contentSize = NSSize(width: 360, height: 400)
        popover.behavior = .transient
        popover.animates = true
        popover.contentViewController = NSHostingController(rootView: MenuBarView())

        self.popover = popover
    }

    @objc private func togglePopover(_ sender: AnyObject?) {
        guard let button = statusItem?.button else { return }

        if let event = NSApp.currentEvent {
            // Right click shows context menu
            if event.type == .rightMouseUp {
                showContextMenu(from: button)
                return
            }
        }

        // Left click toggles popover
        if let popover = popover {
            if popover.isShown {
                popover.performClose(sender)
            } else {
                popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)

                // Ensure popover becomes key window
                popover.contentViewController?.view.window?.makeKey()
            }
        }
    }

    private func showContextMenu(from button: NSStatusBarButton) {
        let menu = NSMenu()

        menu.addItem(withTitle: "Open Dashboard", action: #selector(openDashboard), keyEquivalent: "d")
        menu.addItem(.separator())
        menu.addItem(withTitle: "Settings...", action: #selector(openSettings), keyEquivalent: ",")
        menu.addItem(.separator())
        menu.addItem(withTitle: "Quit OISP", action: #selector(quitApp), keyEquivalent: "q")

        // Set target for menu items
        for item in menu.items {
            item.target = self
        }

        statusItem?.menu = menu
        statusItem?.button?.performClick(nil)
        statusItem?.menu = nil
    }

    // MARK: - Actions

    @objc private func openDashboard() {
        // Open web dashboard
        if let url = URL(string: "http://localhost:3000") {
            NSWorkspace.shared.open(url)
        }
    }

    @objc private func openSettings() {
        // Open settings window
        NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)

        // Bring app to front
        NSApp.activate(ignoringOtherApps: true)
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }

    // MARK: - Event Monitor

    private func setupEventMonitor() {
        eventMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
            if self?.popover?.isShown == true {
                self?.popover?.performClose(event)
            }
        }
    }

    // MARK: - First Launch

    private func checkFirstLaunch() {
        let hasLaunchedBefore = UserDefaults.standard.bool(forKey: "hasLaunchedBefore")

        if !hasLaunchedBefore {
            UserDefaults.standard.set(true, forKey: "hasLaunchedBefore")
            // Show onboarding
            showOnboarding()
        }
    }

    private func showOnboarding() {
        // Will show setup window for extension approval and CA trust
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            self.showSetupWindow()
        }
    }

    private func showSetupWindow() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 500, height: 400),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        window.title = "Welcome to OISP"
        window.center()
        window.contentViewController = NSHostingController(rootView: SetupView())
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    // MARK: - Status Updates

    func updateStatusIcon(isCapturing: Bool) {
        DispatchQueue.main.async {
            if let button = self.statusItem?.button {
                if isCapturing {
                    button.image = NSImage(systemSymbolName: "brain.fill", accessibilityDescription: "OISP - Capturing")
                } else {
                    button.image = NSImage(systemSymbolName: "brain", accessibilityDescription: "OISP - Paused")
                }
            }
        }
    }
}
