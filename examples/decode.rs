use std::fs::File;

fn main() -> Result<(), std::io::Error> {
    let mut decoder = zen_jpeg::Decoder::new(File::open("./test-images/profile.jpg")?);

    decoder.decode().unwrap();

    Ok(())
}
