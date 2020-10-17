use winrt::build;

fn main() {
    build!(
        dependencies
            os
        types
            windows::devices::enumeration::*
            windows::media::capture::MediaCapture
    );
}
