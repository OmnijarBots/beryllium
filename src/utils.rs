use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

pub fn read_file_contents<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    let mut buf = vec![];
    let mut fd = File::open(&path)
                      .map(BufReader::new)
                      .map_err(|e| format!("Cannot open {} ({})", path.as_ref().display(), e))?;
    fd.read_to_end(&mut buf)
      .map_err(|e| format!("Cannot read {} ({})", path.as_ref().display(), e))?;
    Ok(buf)
}

pub fn write_to_file<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<(), String> {
    let mut fd = File::create(&path)
                      .map(BufWriter::new)
                      .map_err(|e| format!("Cannot create {} ({})", path.as_ref().display(), e))?;
    fd.write_all(buf).map_err(|e| format!("Cannot write to {} ({})", path.as_ref().display(), e))
}
