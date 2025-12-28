// Build script for OISP Redirector
// Sets Windows executable metadata using winres

fn main() {
    // Only run on Windows
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../windows/OISPApp/Resources/oisp-icon.ico");
        // Metadata is read from Cargo.toml [package.metadata.winres]
        if let Err(e) = res.compile() {
            // Don't fail the build if icon is missing
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
