use errors::{BerylliumError, BerylliumResult};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

pub fn read_file_contents<P: AsRef<Path>>(path: P) -> BerylliumResult<Vec<u8>> {
    let mut buf = vec![];
    let mut fd = File::open(&path).map(BufReader::new).map_err(BerylliumError::from)?;
    fd.read_to_end(&mut buf).map_err(BerylliumError::from)?;
    Ok(buf)
}

pub fn write_to_file<P: AsRef<Path>>(path: P, buf: &[u8]) -> BerylliumResult<()> {
    let mut fd = File::create(&path).map(BufWriter::new).map_err(BerylliumError::from)?;
    fd.write_all(buf).map_err(BerylliumError::from)
}
