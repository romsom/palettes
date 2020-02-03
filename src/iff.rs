use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::fmt;

#[derive(Debug)]
pub enum Container {
	FORM {
		fourcc: String,
	}
}

#[derive(Debug)]
pub enum ChunkContent {
	GenericChunk { data : Vec<u8> },
	Container {
		sub_chunks: Vec<IFFChunk>,
		container: Container,
	},
	BMHD {
		width: u16,
		height: u16,
		x_origin: i16,
		y_origin: i16,
		n_planes: u8,
		mask: u8,
		compression: u8,
		// pad1: u8,
		transparent_color: u16,
		x_aspect: u8,
		y_aspect: u8,
		page_width: i16,
		page_height: i16,
	},
	CMAP {
		n_colors: usize,
		colors: Vec<(u8, u8, u8)>
	},
	DPPS,
	CRNG,
	TINY,
	BODY,
}

impl fmt::Display for ChunkContent {
	fn fmt(&self, f : & mut fmt::Formatter) -> fmt::Result {
		match self {
			ChunkContent::BMHD { .. } => fmt::Debug::fmt(self, f),
			ChunkContent::CMAP { .. } => fmt::Debug::fmt(self, f), //write!(f, "CMAP {{ .. }}"),
			ChunkContent::GenericChunk { .. } => write!(f, "GenericChunk {{ data }}"),
			ChunkContent::Container { .. } => write!(f, "Container {{ .. }}"),
			ChunkContent::DPPS { .. } => write!(f, "DPPS {{ .. }}"),
			ChunkContent::CRNG { .. } => write!(f, "CRNG {{ .. }}"),
			ChunkContent::TINY { .. } => write!(f, "TINY {{ .. }}"),
			ChunkContent::BODY { .. } => write!(f, "BODY {{ .. }}"),
		}
	}
}

#[derive(Debug)]
pub struct IFFChunk {
	pub chunk_type: String,
	pub size: usize,
	pub enumeration_complete: bool,
	pub chunk_number: Option<usize>,
	pub data: ChunkContent,
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
		// TODO initialize fixed size array from slice

		// parse chunk type
		let header_size : usize;// = 8;
		let mut chunk_tmp : [u8; 4] = [0, 0, 0, 0];
		chunk_tmp.copy_from_slice(&bytes[..4]);
		let chunk_type = IFFChunk::parse_4_bytes(chunk_tmp);

		// parse chunk length
		let mut data_size_bytes : [u8; 4] = Default::default();
		data_size_bytes.copy_from_slice(&bytes[4..8]);
		let mut data_size : usize = u32_from_be_bytes(data_size_bytes) as usize;

		// special case for FORM chunk
		if chunk_type == "FORM" && data_size >= 4 {
				header_size = 12;
				data_size -= 4;
		} else {
			header_size = 8;
		}

		//print!("Found chunk with type: {}\n", chunk_type);
		//print!("Chunk size {}\n", data_size);
		//print!("Remaining bytes: {}\n", bytes.len());

		// read data bytes
		let mut data_bytes : Vec<u8> = Vec::with_capacity(data_size);
		// initially fill data with zeros
		data_bytes.resize_with(data_size, Default::default);
		if header_size + data_size <= bytes.len() {
			data_bytes.copy_from_slice(&bytes[header_size..header_size+data_size]);
		} else {
			data_bytes.copy_from_slice(&bytes[header_size..]);
			print!("Truncated chunk: there were only {} bytes where there should have been {}\n",
				   bytes.len() - header_size,
				   data_size);
		}


		// create data field depending on chunk type
		
		let data = if chunk_type == "FORM" {
			// read fourcc field
			let mut fourcc_tmp : [u8; 4] = [0, 0, 0, 0];
			fourcc_tmp.copy_from_slice(&bytes[8..12]);
			// parse subchunks
			let mut sub_chunks : Vec<IFFChunk> = Vec::new();
			IFFChunk::find_chunks(& data_bytes, & mut sub_chunks);
			// build struct
			ChunkContent::Container {
				sub_chunks : sub_chunks,
				container : Container::FORM {
					fourcc : IFFChunk::parse_4_bytes(fourcc_tmp),
				}
			}
		} else if chunk_type == "BMHD" { // bitmap header

			ChunkContent::BMHD {
				width: u16_from_be_bytes([data_bytes[0], data_bytes[1]]),
				height: u16_from_be_bytes([data_bytes[2], data_bytes[3]]),
				x_origin: i16_from_be_bytes([data_bytes[4], data_bytes[5]]),
				y_origin: i16_from_be_bytes([data_bytes[6], data_bytes[7]]),
				n_planes: data_bytes[8],
				mask: data_bytes[9],
				compression: data_bytes[10],
				// pad1: u8 = data_bytes[11],
				transparent_color: u16_from_be_bytes([data_bytes[12], data_bytes[13]]),
				x_aspect: data_bytes[14],
				y_aspect: data_bytes[15],
				page_width: i16_from_be_bytes([data_bytes[16], data_bytes[17]]),
				page_height: i16_from_be_bytes([data_bytes[18], data_bytes[19]]),
			}
		} else if chunk_type == "CMAP" {
			let n_colors = data_size / 3;
			// print!("Found CMAP with {} colors.\n", data_size / 3);
			let mut colors = Vec::<(u8, u8, u8)>::with_capacity(data_size / 3);
			for i in 0..n_colors {
				colors.push((data_bytes[3*i], data_bytes[3*i+1], data_bytes[3*i+2]));
			}
			ChunkContent::CMAP {
				n_colors: n_colors,
				colors: colors,
			}
			// TODO add more specific chunk types
		} else {
			ChunkContent::GenericChunk {
				data : data_bytes
			}
		};

		(IFFChunk {
			chunk_type: chunk_type,
			size: data_size,
			enumeration_complete: false,
			chunk_number: None,
			data: data,
		}, header_size + data_size)
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
			// recurse over children if any
			match & mut c.data {
				ChunkContent::Container {sub_chunks : scs, .. } => {
					for sc in scs {
						next = IFFChunk::enumerate_rec(sc, next, level - 1);
					}
				},
				_ => (),
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
		write!(f, "Chunk {}, Type: {}, Size: {}, Content: {}", self.chunk_number.unwrap(), self.chunk_type, self.size, self.data)
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

fn u16_from_be_bytes(bytes: [u8; 2]) -> u16 {
	((bytes[0] as u16) << 8) + (bytes[1] as u16)
}

fn i16_from_be_bytes(bytes: [u8; 2]) -> i16 {
	((bytes[0] as i16) << 8) + (bytes[1] as i16)
}
