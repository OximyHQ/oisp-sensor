// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "OISP",
    platforms: [
        .macOS(.v13) // Ventura minimum for NETransparentProxyProvider improvements
    ],
    products: [
        // OISPCore - Shared library for models, networking, IPC
        .library(
            name: "OISPCore",
            targets: ["OISPCore"]
        ),
        // OISPNetworkExtensionLib - Network extension components (library)
        .library(
            name: "OISPNetworkExtensionLib",
            targets: ["OISPNetworkExtensionLib"]
        ),
    ],
    dependencies: [
        // No external dependencies for now
        // In production, consider:
        // .package(url: "https://github.com/apple/swift-certificates.git", from: "1.0.0"),
    ],
    targets: [
        // MARK: - OISPCore
        .target(
            name: "OISPCore",
            dependencies: [],
            path: "OISPCore/Sources",
            linkerSettings: [
                .linkedLibrary("bsm")
            ]
        ),
        .testTarget(
            name: "OISPCoreTests",
            dependencies: ["OISPCore"],
            path: "OISPCore/Tests"
        ),

        // MARK: - OISPNetworkExtensionLib (library without main.swift)
        .target(
            name: "OISPNetworkExtensionLib",
            dependencies: ["OISPCore"],
            path: "OISPNetworkExtension",
            exclude: ["main.swift"]
        ),
    ]
)
