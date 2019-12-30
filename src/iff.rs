use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

struct IFFChunk<'a> {
	chunk_type: [char; 4],
	length: u32, // TODO big endian conversion
	data: & 'a mut [u8] // reference to other memory
}

struct IFFFile<'a> {
	chunks: Vec<& 'a mut IFFChunk<'a>>,
}

impl<'a> IFFFile<'a> {
	fn new(chunks: Vec<& 'a mut IFFChunk<'a>>) -> IFFFile<'a> {
		IFFFile {
			chunks: chunks,
		}
	}

	fn read_from_file(path: &Path) -> IFFFile<'a> {
		// open file
		let mut file = match File::open(path) {
			Err(why) => panic!("Could not open {}: {}",
								 path.display(),
								 why.description()),
			Ok(file) => file,
		};
		// read file contents
		let mut bytes = Vec::new();
		let n_bytes = match file.read_to_end(&mut bytes) {
			Err(why) => panic!("Could not read {}: {}",
							   path.display(),
							   why.description()),
			Ok(n) => n,
		};
		// parse file
		let mut chunks = Vec::<& mut IFFChunk>::new();
		loop {
			
		}
		let iff_file = IFFFile::new(Vec::new());
	}
}
