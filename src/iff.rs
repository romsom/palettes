use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

struct IFFChunk<'a> {
	chunk_type: [u8; 4],
	data: & 'a mut [u8] // reference to other memory
}

impl<'a> IFFChunk<'a> {
	fn parse(bytes: & 'a mut [u8]) -> (IFFChunk<'a>, usize) {
		// initialize fixed size array from slice
		let mut chunk_type : [u8; 4] = Default::default();
		chunk_type.copy_from_slice(&bytes[..4]);
		let mut data_size_bytes : [u8; 4] = Default::default();
		data_size_bytes.copy_from_slice(&bytes[4..8]);
		let data_size : usize = u32FromBEBytes(data_size_bytes) as usize;
		let data = & mut bytes[8..8+data_size];
		(IFFChunk { chunk_type: chunk_type, data: data }, 8 + data_size)
	}
}

struct IFFFile<'a> {
	chunks: Vec<& 'a IFFChunk<'a>>,
}

impl<'a> IFFFile<'a> {
	fn new(chunks: Vec<& 'a IFFChunk<'a>>) -> IFFFile<'a> {
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
		let mut chunks = Vec::<& IFFChunk>::new();
		let mut offset : usize = 0;
		loop {
			if offset + 8 > n_bytes {
				break;
			}
			let (chunk, chunk_size) = IFFChunk::parse(& mut bytes[offset..]);
			chunks.push(& chunk);
			if chunk_size % 2 == 0 {
				offset += chunk_size;
			} else {
				offset += chunk_size + 1;
			}
		}
		IFFFile::new(chunks)
	}
}

fn u32FromBEBytes(bytes: [u8; 4]) -> u32 {
	bytes[3] as u32 + (bytes[2] as u32) << 8 + (bytes[1] as u32) << 16 + (bytes[0] as u32) << 24
}
