//! Reader and writer for the RPG Maker XYZ image format.
//!
//! # Example
//!
//! This library works well with the [`image`][1] crate.
//!
//! Here's an example of reading an XYZ image into an [`ImageBuffer`][2]:
//!
//! ```ignore
//! extern crate image;
//! extern crate xyz;
//!
//! use image::RgbImage;
//! use std::fs::File;
//!
//! let file = try!(File::open("boat2.xyz"));
//! let raw = try!(xyz::read(&mut file));
//! let boat = RgbImage::from_raw(raw.width as u32, raw.height as u32, raw.to_rgb_buffer());
//! ```
//!
//! You can then do something useful with the `boat`.
//!
//! [1]: https://github.com/PistonDevelopers/image
//! [2]: http://www.piston.rs/image/image/struct.ImageBuffer.html

extern crate byteorder;
extern crate flate2;

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use flate2::{FlateReadExt, FlateWriteExt, Compression};
use std::io::{self, Read, Write};

const MAGIC_NUMBER: u32 = 0x315a5958;  // "XYZ1"

/// Represents an XYZ image.
pub struct Image {
    /// Image height in pixels.
    pub width: u16,
    /// Image width in pixels.
    pub height: u16,
    /// List of colors used by the image.
    pub palette: [Rgb; 256],
    /// Image data. This contains `width * height` bytes, one for each
    /// pixel in the image. The color of each pixel is determined by the
    /// `palette` array.
    pub buffer: Vec<u8>,
}

/// Represents a color in RGB form.
pub type Rgb = [u8; 3];

impl Image {
    /// Converts the image to a raw RGB buffer, suitable for the Piston
    /// `image` library.
    pub fn to_rgb_buffer(&self) -> Vec<u8> {
        self.buffer.iter()
            .flat_map(|&i| self.palette[i as usize].iter().cloned())
            .collect()
    }
}

/// Reads an XYZ image.
pub fn read<R: Read>(reader: &mut R) -> io::Result<Image> {
    let magic = try!(reader.read_u32::<LittleEndian>());
    if magic != MAGIC_NUMBER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid XYZ header"));
    }

    let width = try!(reader.read_u16::<LittleEndian>());
    let height = try!(reader.read_u16::<LittleEndian>());

    let mut decompress = vec![].zlib_decode();
    try!(io::copy(reader, &mut decompress));
    let body = try!(decompress.finish());
    let mut body = &body as &[u8];

    let mut palette = [[0u8; 3]; 256];
    for slot in palette.iter_mut() {
        try!(body.read_exact(slot));
    }

    let mut buffer = vec![0u8; (width as usize) * (height as usize)];
    try!(body.read_exact(&mut buffer));

    if !body.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "extra data at end of XYZ file"));
    }

    Ok(Image {
        width: width,
        height: height,
        palette: palette,
        buffer: buffer,
    })
}

/// Writes an XYZ image.
pub fn write<W: Write>(image: &Image, writer: &mut W) -> io::Result<()> {
    try!(writer.write_u32::<LittleEndian>(MAGIC_NUMBER));

    try!(writer.write_u16::<LittleEndian>(image.width));
    try!(writer.write_u16::<LittleEndian>(image.height));

    let mut compress = writer.zlib_encode(Compression::Default);
    for slot in image.palette.iter() {
        try!(compress.write_all(slot));
    }
    try!(compress.write_all(&image.buffer));
    try!(compress.finish());

    Ok(())
}
