use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::fmt;
use std::mem;

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
	CRNG {
		rate: i16,
		flags: u16,
		active: bool,
		cycle_downwards: bool,
		low: u8,
		high: u8,
	},
	TINY,
	BODY {
		raw_data: Vec<u8>,
		decompressed_data: Option<Vec<u8>>,
		pixel_data: Option<Vec<u8>>
	},
}

impl fmt::Display for ChunkContent {
	fn fmt(&self, f : & mut fmt::Formatter) -> fmt::Result {
		match self {
			ChunkContent::BMHD { .. } => fmt::Debug::fmt(self, f),
			ChunkContent::CMAP { .. } => fmt::Debug::fmt(self, f), //write!(f, "CMAP {{ .. }}"),
			ChunkContent::CRNG { .. } => fmt::Debug::fmt(self, f),
			ChunkContent::GenericChunk { .. } => write!(f, "GenericChunk {{ data }}"),
			ChunkContent::Container { container, .. } => {
				write!(f, "Container {{ ").and(
					fmt::Display::fmt(container, f).and(
						write!(f, "}}")))
			},
			// ChunkContent::Container { .. } => fmt::Debug::fmt(self, f),
			ChunkContent::DPPS { .. } => write!(f, "DPPS {{ .. }}"),
			ChunkContent::TINY { .. } => write!(f, "TINY {{ .. }}"),
			ChunkContent::BODY { .. } => write!(f, "BODY {{ .. }}"),
		}
	}
}

impl fmt::Display for Container {
	fn fmt(&self, f : & mut fmt::Formatter) -> fmt::Result {
		match self {
			Container::FORM { fourcc } => write!(f, "FORM {{ fourcc: {} }}", fourcc),
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
			let colors = (0..n_colors).map(|i| (data_bytes[3*i], data_bytes[3*i+1], data_bytes[3*i+2])).collect::<Vec::<(u8, u8, u8)>>();
			// let mut colors = Vec::<(u8, u8, u8)>::with_capacity(data_size / 3);
			// for i in 0..n_colors {
			// 	colors.push((data_bytes[3*i], data_bytes[3*i+1], data_bytes[3*i+2]));
			// }
			ChunkContent::CMAP {
				n_colors: n_colors,
				colors: colors,
			}
		} else if chunk_type == "CRNG" {
			// data_bytes[0..2]: padding
			let flags = u16_from_be_bytes([data_bytes[4], data_bytes[5]]);
			ChunkContent::CRNG {
				rate: i16_from_be_bytes([data_bytes[2], data_bytes[3]]),
				flags: flags,
				active: (flags & 0x1) != 0,
				cycle_downwards: (flags & 0x2) != 0,
				low: data_bytes[6],
				high: data_bytes[7],
			}
		} else if chunk_type == "BODY" {
			ChunkContent::BODY {
				raw_data: data_bytes,
				decompressed_data: None,
				pixel_data: None,
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

	fn decompress_body(data: & Vec<u8>, dest: & mut Vec<u8>) {
		// row end is a compression boundary
		let mut i: usize = 0;
		loop {
			if i >= data.len() {
				break;
			}
			
			if data[i] == 0x80 { // NOP
				i += 1;
			} else if data[i] < 0x80 { // data[i] different bytes
				let next_i = i + (data[i] as usize) + 1;
				for j in i+1..next_i {
					dest.push(data[j])
				}
				i = next_i;
			} else { // same byte data[i] - 0x80 times
				let n : usize = (data[i] as usize) - 0x80;
				for _ in 0..n {
					dest.push(data[i+1])
				}
				i += n + 1
			}
		}
	}

	fn decode_body(data: & Vec<u8>, dest: &mut Vec<u8>, n_pixel_planes: u8, mask_plane: bool) {
		assert!(n_pixel_planes <= 8); // could potentially be more, but we assume the resulting index to be a u8
		let mut i = 0;
		let n_planes = if mask_plane {
			(n_pixel_planes + 1) as usize
		} else {
			n_pixel_planes as usize
		};
		loop {
			// i is the next unused index
			for _ in 0..8 {
				dest.push(0);
			}
			// i .. i + width - 1 are now valid indices
			for plane_index in 0..n_pixel_planes as usize {
				let mut plane : u8 = data[(i * n_planes) + plane_index];
				for pixel in 0..8 {
					//let bit : u8 = (data[(i * n_planes) + plane_index] >> pixel) & 0x1;
					//dest[(i * 8) + pixel] |= bit << plane_index;

					dest[(i * 8) + pixel] |= (plane & 0x1) << plane_index;
					plane >>= 1;
				}
			}
			
			i += 1;
		}
	}

	pub fn prepare_body(mut self, bmhd: & IFFChunk) -> Result<Self, & 'static str> {
		match self.data {
			ChunkContent::BODY { raw_data, decompressed_data, pixel_data } => {
				match bmhd.data {
					ChunkContent::BMHD { n_planes, mask, .. } => {

						let decompress_buffer = match decompressed_data {
							None => {
								let mut decompress_buffer = Vec::<u8>::new();
								IFFChunk::decompress_body(& raw_data, & mut decompress_buffer);
								decompress_buffer
							}
							Some(decompressed) => decompressed
						};

						let pixel_buffer = match pixel_data {
							None => {
								let mut pixel_buffer = Vec::<u8>::new();
								IFFChunk::decode_body(& decompress_buffer, & mut pixel_buffer, n_planes, (mask & 0x1) != 0);
								pixel_buffer
							},
							Some(pixels) => pixels
						};
						
						self.data = ChunkContent::BODY {
							raw_data: raw_data,
							decompressed_data: Some(decompress_buffer),
							pixel_data: Some(pixel_buffer)
						};
						Ok(self)
					},
					_ => Err("bmhd must be of type BMHD (bitmap header)")
				}
			},
			_ => Err("Chunk must be of type BODY to call this function on.")
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
		let bmhd_addr = IFFFile::find_chunk(& mut chunks, & String::from("BMHD")).unwrap();
		let body_addr = IFFFile::find_chunk(& mut chunks, & String::from("BODY")).unwrap();
		print!("{:?}\n", bmhd_addr);
		print!("{:?}\n", body_addr);
		IFFFile {
			chunks: chunks
		}
	}

	// find first matching chunk if any
	pub fn find_chunk(chunks: & Vec<IFFChunk>, chunk_type: & String) -> Result<Vec<usize>, & 'static str> {
		for (i, ch) in chunks.iter().enumerate() {
			if & ch.chunk_type == chunk_type {
				return Ok(vec![i])
			} else {
				match & ch.data {
					ChunkContent::Container { sub_chunks, .. } => {
						match IFFFile::find_chunk( & sub_chunks, & chunk_type) {
							Ok(mut ch) => {
								ch.insert(0, i);
								return Ok(ch);
							}
							_ => ()
						}
					},
					_ => ()
				}
			}
		}
		Err("No chunk with matching type was found.")
	}

	pub fn update_body(chunks: Vec<IFFChunk>, bmhd_addr: Vec<usize>, body_addr: Vec<usize>) -> Vec<IFFChunk> {
		// This approach does not work unless both Vecs have exactly the same length, i.e. bmhd and body lie in the same container (which probably is the case in most if not all cases, but it's not a general solution)
		// I have to rethink this and find a way to modify existing structures
		chunks
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
