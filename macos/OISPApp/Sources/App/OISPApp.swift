// OISPApp.swift
// OISPApp
//
// Main entry point for the OISP menu bar application

import SwiftUI

@main
struct OISPApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        // Menu bar only app - no main window
        Settings {
            SettingsView()
        }
    }
}
