use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::fmt;
//use std::default::Default;

#[derive(Debug)]
pub struct IFFChunk {
	pub chunk_type: String,
	pub data: Vec<u8>, // TODO: reference to other memory
	pub sub_chunks: Vec<IFFChunk>,
	pub fourcc: Option<String>,
	pub enumeration_complete: bool,
	pub chunk_number: Option<usize>,
}

impl IFFChunk {
	fn parse_4_bytes(ct: [u8; 4]) -> String {
		String::from_utf8(ct.to_vec()).unwrap()
	}

	fn find_chunks(data: & Vec<u8>, chunks : & mut Vec<IFFChunk>) {
		let mut offset : usize = 0;
		let n_bytes : usize = data.len();
		loop {
			if offset + 8 > n_bytes {
				break;
			}
			let (chunk, chunk_size) = IFFChunk::parse(& data[offset..]);
			chunks.push(chunk);
			if chunk_size % 2 == 0 {
				offset += chunk_size;
			} else {
				offset += chunk_size + 1;
			}
		}
	}

	fn parse(bytes: & [u8]) -> (IFFChunk, usize) {
		// initialize fixed size array from slice

		let header_size : usize;// = 8;
		let mut chunk_tmp : [u8; 4] = [0, 0, 0, 0];
		chunk_tmp.copy_from_slice(&bytes[..4]);
		let chunk_type = IFFChunk::parse_4_bytes(chunk_tmp);

		let mut data_size_bytes : [u8; 4] = Default::default();
		data_size_bytes.copy_from_slice(&bytes[4..8]);
		let mut data_size : usize = u32_from_be_bytes(data_size_bytes) as usize;

		let fourcc =
			if chunk_type == "FORM" && data_size >= 4 {
				let mut fourcc_tmp : [u8; 4] = [0, 0, 0, 0];
				fourcc_tmp.copy_from_slice(&bytes[8..12]);
				Some(IFFChunk::parse_4_bytes(fourcc_tmp))
			} else {
				None
			};

		//print!("Found chunk with type: {}\n", chunk_type);
		match & fourcc {
			Some(fourcc_str) => {
				header_size = 12;
				data_size -= 4;
				//print!("Chunk fourcc: {}\n", fourcc_str)
			},
			None => {
				header_size = 8;
				()
			}
		}
		//print!("Chunk size {}\n", data_size);

		//print!("Remaining bytes: {}\n", bytes.len());

		let mut data : Vec<u8> = Vec::with_capacity(data_size);
		// fill data with zeros
		data.resize_with(data_size, Default::default);
		if header_size + data_size <= bytes.len() {
			data.copy_from_slice(&bytes[header_size..header_size+data_size]);
		} else {
			data.copy_from_slice(&bytes[header_size..]);
			// print!("Truncated chunk: there were only {} bytes where there should have been {}\n",
			//	   bytes.len() - header_size,
			//	   data_size);
		}
		let mut sub_chunks : Vec<IFFChunk> = Vec::new();
		if chunk_type == "FORM" {
			IFFChunk::find_chunks(& data, & mut sub_chunks);
		}
		(IFFChunk { chunk_type: chunk_type, data: data, sub_chunks : sub_chunks, fourcc : fourcc, enumeration_complete: false, chunk_number: None }, 8 + data_size)
	}

	// enumerate chunks bredth-first
	pub fn enumerate(cs: & mut Vec<IFFChunk>) {
		let mut next : usize = 0;
		let mut old : usize;
		let mut level = 0;
		loop {
			old = next;
			for c in & mut *cs {
				next = IFFChunk::enumerate_rec(c, next, level);
			}
			level += 1;
			if old == next {
				break;
			}
		}
	}

	fn enumerate_rec(c : & mut IFFChunk, next_index : usize, level : usize) -> usize {
		if c.enumeration_complete {
			next_index
		} else if level == 0 {
			c.chunk_number = Some(next_index);
			next_index + 1
		} else {
			let mut next = next_index;
			for sc in & mut c.sub_chunks {
				next = IFFChunk::enumerate_rec(sc, next, level - 1);
			}
			if next_index == next {
				c.enumeration_complete = true;
			}
			next
		}
	}
}

impl std::fmt::Display for IFFChunk {
	fn fmt(&self, f : & mut fmt::Formatter) -> fmt::Result {
		write!(f, "Chunk {}, Type: {}, Size {}", self.chunk_number.unwrap(), self.chunk_type, self.data.len())
	}
}

pub struct IFFFile {
	pub chunks: Vec<IFFChunk>,
}

impl IFFFile {
	pub fn read_from_file(path: &Path) -> IFFFile {
		// open file
		let mut file = match File::open(path) {
			Err(why) => panic!("Could not open {}: {}",
								 path.display(),
								 why.description()),
			Ok(file) => file,
		};
		// read file contents
		let mut bytes = Vec::new();
		// TODO: optimization: try to read 8 bytes for type and size, then size bytes, then repeat
		let _n_bytes = match file.read_to_end(&mut bytes) {
			Err(why) => panic!("Could not read {}: {}",
							   path.display(),
							   why.description()),
			Ok(n) => n,
		};
		// parse file
		let mut chunks = Vec::<IFFChunk>::new();
		IFFChunk::find_chunks(& bytes, & mut chunks);
		IFFChunk::enumerate(& mut chunks);
		IFFFile {
			chunks: chunks
		}
	}
}

fn u32_from_be_bytes(bytes: [u8; 4]) -> u32 {
	bytes[3] as u32 + ((bytes[2] as u32) << 8) + ((bytes[1] as u32) << 16) + ((bytes[0] as u32) << 24)
}
