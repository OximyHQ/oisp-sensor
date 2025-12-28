// main.swift
// OISPNetworkExtension
//
// Entry point for the Network Extension system extension

import Foundation
import NetworkExtension

// The extension principal class is defined in Info.plist
// This file provides a minimal entry point

autoreleasepool {
    NEProvider.startSystemExtensionMode()
}

// Keep the extension running
dispatchMain()
