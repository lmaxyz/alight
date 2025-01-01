use winresource::WindowsResource;

fn main() {
    WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/icon.ico")
            .compile().unwrap();

    slint_build::compile("src/ui/main.slint").unwrap();
}