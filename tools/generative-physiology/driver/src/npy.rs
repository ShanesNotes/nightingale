use std::fs::{self, File};
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::path::Path;

/// NPY format 1.0: the magic, the version and a little-endian header length, then the header text. A fixed 128
/// bytes keeps the data 64-byte aligned and lets `finish` rewrite the row count in place.
const HEADER: usize = 128;
const MAGIC: &[u8; 8] = b"\x93NUMPY\x01\x00";

pub trait Element: Copy {
    const DESCR: &'static str;
    fn to_le(self) -> [u8; 8];
    fn from_le(bytes: [u8; 8]) -> Self;
}
impl Element for u64 {
    const DESCR: &'static str = "<u8";
    fn to_le(self) -> [u8; 8] {
        self.to_le_bytes()
    }
    fn from_le(bytes: [u8; 8]) -> Self {
        Self::from_le_bytes(bytes)
    }
}
impl Element for f64 {
    const DESCR: &'static str = "<f8";
    fn to_le(self) -> [u8; 8] {
        self.to_le_bytes()
    }
    fn from_le(bytes: [u8; 8]) -> Self {
        Self::from_le_bytes(bytes)
    }
}

fn prefix<E: Element>() -> String {
    format!("{{'descr': '{}', 'fortran_order': False, 'shape': (", E::DESCR)
}

fn header<E: Element>(rows: u64, columns: usize) -> [u8; HEADER] {
    let text = format!("{}{rows}, {columns}), }}", prefix::<E>());
    let mut out = [b' '; HEADER];
    out[..MAGIC.len()].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&((HEADER - 10) as u16).to_le_bytes());
    out[10..10 + text.len()].copy_from_slice(text.as_bytes());
    out[HEADER - 1] = b'\n';
    out
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

/// A two-dimensional array written row by row, so a run's memory does not grow with its length.
pub struct NpyWriter<E> {
    file: BufWriter<File>,
    columns: usize,
    rows: u64,
    element: PhantomData<E>,
}
impl<E: Element> NpyWriter<E> {
    pub fn create(path: &Path, columns: usize) -> io::Result<Self> {
        let mut file = BufWriter::new(File::create_new(path)?);
        file.write_all(&header::<E>(0, columns))?;
        Ok(Self {
            file,
            columns,
            rows: 0,
            element: PhantomData,
        })
    }
    pub fn push(&mut self, row: &[E]) -> io::Result<()> {
        if row.len() != self.columns {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("a row of {} values in an array of {}", row.len(), self.columns),
            ));
        }
        for value in row {
            self.file.write_all(&value.to_le())?;
        }
        self.rows += 1;
        Ok(())
    }
    pub fn rows(&self) -> u64 {
        self.rows
    }
    /// Writes the row count into the header and syncs the file.
    pub fn finish(self) -> io::Result<()> {
        let mut file = self.file.into_inner().map_err(io::IntoInnerError::into_error)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&header::<E>(self.rows, self.columns))?;
        file.sync_all()
    }
}

/// A finished array, row major.
#[derive(Debug, PartialEq)]
pub struct Table<E> {
    columns: usize,
    values: Vec<E>,
}
impl<E: Element> Table<E> {
    pub fn read(path: &Path) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        let (head, data) = bytes
            .split_at_checked(HEADER)
            .ok_or_else(|| invalid("shorter than a header"))?;
        if head[..MAGIC.len()] != MAGIC[..] {
            return Err(invalid("not an NPY 1.0 file"));
        }
        let text = core::str::from_utf8(&head[10..]).map_err(|_| invalid("a header that is not text"))?;
        let shape = (text.strip_prefix(&prefix::<E>()))
            .and_then(|rest| rest.split_once("), }"))
            .and_then(|(shape, _)| shape.split_once(", "))
            .ok_or_else(|| invalid("another element type or layout"))?;
        let rows: usize = shape.0.parse().map_err(|_| invalid("a row count"))?;
        let columns: usize = shape.1.parse().map_err(|_| invalid("a column count"))?;
        let (chunks, rest) = data.as_chunks::<8>();
        if !rest.is_empty() || Some(chunks.len()) != rows.checked_mul(columns) {
            return Err(invalid("a data length the shape does not give"));
        }
        let values = chunks.iter().map(|&chunk| E::from_le(chunk)).collect();
        Ok(Self { columns, values })
    }
    pub fn columns(&self) -> usize {
        self.columns
    }
    pub fn rows(&self) -> usize {
        self.values.len().checked_div(self.columns).unwrap_or(0)
    }
    pub fn row(&self, at: usize) -> &[E] {
        &self.values[at * self.columns..(at + 1) * self.columns]
    }
    pub fn iter(&self) -> impl Iterator<Item = &[E]> + '_ {
        (0..self.rows()).map(|at| self.row(at))
    }
}
