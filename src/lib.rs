use std::{fs::{self, File}, io::{self, ErrorKind, IoSlice, IoSliceMut, Seek, SeekFrom, Read, Write}, ops::{Deref, DerefMut}, path::Path};

pub struct DropFile {
	path: Box<Path>,
	file: Option<File>,
	created: bool,
	written_to: bool,
}
impl DropFile {
	pub fn open<P: AsRef<Path>>(path: P, create: bool) -> Result<Self, &'static str> {
		let path = path.as_ref();

		let mut file_options = File::options();
		file_options.read(true).write(true).create_new(create);
		
		let file = file_options.open(path).map_err(|err| match err.kind() {
			ErrorKind::AlreadyExists => "file already exists",
			_ => "failed to open file"
		})?;

		return Ok(Self { path: path.into(), file: Some(file), created: create, written_to: false, });
	}

	pub fn delete_file(&mut self) -> Result<(), &'static str> {
		if self.file.take().is_none() {
			return Ok(());
		}
		return fs::remove_file(&(self.path)).map_err(|_|
			"failed to delete file"
		);
	}
	pub fn delete(mut self) -> Result<(), &'static str> {
		return self.delete_file();
	}

	pub fn trunc(&mut self) -> Result<(), &'static str> {
		let file = self.deref_mut();
		file.rewind().map_err(|_| "failed to rewind file")?;
		file.set_len(0).map_err(|_| "failed to truncate file")?;

		return Ok(());
	}
	pub fn trunc_to_cursor(&mut self) -> Result<(), &'static str> {
		let file = self.deref_mut();
		let cursor = file.stream_position().map_err(|_| "failed to get cursor position")?;
		file.set_len(cursor).map_err(|_| "failed to truncate file")?;
		return Ok(());
	}

	pub fn path(&self) -> &Path {
		return &(self.path);
	}
}
impl Drop for DropFile {
	fn drop(&mut self) {
		if self.created && !self.written_to {
			self.delete_file().unwrap();
		}
	}
}
impl Deref for DropFile {
	type Target = fs::File; 
	fn deref(&self) -> &Self::Target {
		return self.file.as_ref().unwrap();
	}
}
impl DerefMut for DropFile {
	fn deref_mut(&mut self) -> &mut Self::Target {
		return self.file.as_mut().unwrap();
	}
}

impl Seek for DropFile {
	fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
		return self.deref_mut().seek(pos);
	}
}
impl Read for DropFile {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		return self.deref_mut().read(buf);
	}

	fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
		return self.deref_mut().read_vectored(bufs);
	}

	fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
		return self.deref_mut().read_to_end(buf);
	}

	fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
		return self.deref_mut().read_to_string(buf);
	}
}
impl Write for DropFile {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		return self.deref_mut().write(buf).map(|w| {
			self.written_to = true;
			return w;
		});
	}

	fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
		return self.deref_mut().write_vectored(bufs).map(|w| {
			self.written_to = true;
			return w;
		});
	}

	fn flush(&mut self) -> io::Result<()> {
		return self.deref_mut().flush().map(|_| {
			self.written_to = true;
			return ();
		});
	}
}

#[cfg(test)]
mod tests {
	use {super::*, std::{assert_eq, io::Read, io::SeekFrom, path::Path}};

	#[test]
	fn new() {
		drop(fs::remove_file("/tmp/dropfile"));

		let mut file = DropFile::open("/tmp/dropfile", true).unwrap();
		file.write("abcd".as_bytes()).unwrap();
		drop(file);

		let Err(_) = DropFile::open("/tmp/dropfile", true) else {
			panic!("/tmp/dropfile created despite already existing");
		};

		let mut file = DropFile::open("/tmp/dropfile", false).unwrap();
		let mut buffer = [0u8; 3];
		file.seek(SeekFrom::Start(1)).unwrap();
		file.read_exact(&mut(buffer)).unwrap();
		assert_eq!(buffer, "bcd".as_bytes());
		file.trunc().unwrap();
		drop(file);

		let mut file = DropFile::open("/tmp/dropfile", false).unwrap();
		let file_len = file.seek(SeekFrom::End(0)).unwrap();
		assert_eq!(file_len, 0);
		file.delete().unwrap();

		let file = DropFile::open("/tmp/dropfile", true).unwrap();
		drop(file);

		assert!(!Path::new("/tmp/dropfile").exists());
	}
}
