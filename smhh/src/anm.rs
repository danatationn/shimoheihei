use std::{collections::HashMap, fmt, fs::File, io::{Read, Seek, SeekFrom}, path::Path};

use thiserror::Error;

use crate::ReadExt;

/// Animation Script

pub struct Anm {
	pub magic: [u8; 4],
	pub timeline_count: u32,
	pub timelines: Vec<AnmTimeline>,
}

#[derive(Error, Debug)]
pub enum AnmError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Timeline error: {0}")]
	AnmTimelineError(#[from] AnmTimelineError),
	#[error("Parameter error: {0}")]
	AnmParameterError(#[from] AnmParameterError),
	#[error("Invalid signature: {0:?}")]
	InvalidSignature([u8; 4]),
}

impl Anm {
	pub fn from_path<P: AsRef<Path>>(path: &P) -> Result<Self, AnmError> {
		let mut file = File::open(path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, AnmError> {
		let mut magic = [0u8; 4];
		reader.read_exact(&mut magic)?;
		if &magic != b"ANM\0" {
			return Err(AnmError::InvalidSignature(magic))
		}

		reader.seek(SeekFrom::Current(4))?;  // skipping unknown u32

		let timeline_count: u32 = reader.read_le::<u32>()?;

		reader.seek(SeekFrom::Current(20))?;  // skipping unknown 20 bytes

		let mut timelines = Vec::with_capacity(timeline_count as usize);
		for _ in 0..timeline_count {
			let timeline = AnmTimeline::from(reader)?;
			timelines.push(timeline);
		}

		Ok(Self{ magic, timeline_count, timelines })
	}

	pub fn decompile(&self) -> Result<Vec<u8>, AnmError> {
		let mut script = String::new();

		// GetLabelMap
		let mut label_vec: Vec<u32> = Vec::new();

		for timeline in &self.timelines {
			if timeline.command_type == AnmCommandType::Jump {
				let parameter = timeline.parameters.get(timeline.parameters.len() - 1)
			        .unwrap();  // todo: replace with unwrap_or_else

				if parameter.parameter_type == AnmParameterType::Variable {
					label_vec.push(parameter.value);
				}
			}
		}

		let mut label_index = 0;
		let mut label_map: HashMap<u32, String> = HashMap::new();
		label_vec.sort();

		for i in &label_vec {
			label_map.insert(*i, label_index.to_string());
			label_index += 1;
		}

		// DecompileInternal
		for (i, timeline) in self.timelines.iter().enumerate() {
			if let Some(label_name) = label_map.get(&(i as u32)) {
				script += &format!("#{label_name}");
			}

			script += &format!("\t{}", timeline.command_type.as_str());
			if !timeline.command_type.as_str().is_empty() {
				script.push(' ');
			}

			for (i, parameter) in timeline.parameters.iter().enumerate() {
				script += &format!("{}", parameter.value.to_string());
				if timeline.parameters.len()-1 != i {
					script.push(' ');
				} else {
					script.push('\n');
				}
			}
		}

		Ok(script.into_bytes())
	}
}

impl fmt::Debug for Anm {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "Animation file with {} timelines", self.timeline_count)?;
		for i in 0..self.timeline_count {
			let timeline = &self.timelines[i as usize];

			writeln!(f, "{i}. {:?}: {:?}", timeline.command_type, timeline.parameters)?;
		}
		Ok(())
	}
}

pub struct AnmTimeline {
	pub command_type: AnmCommandType,
	pub parameters: Vec<AnmParameter>,  // technically there can be 8 params but only 3 get used
}

#[derive(Error, Debug)]
pub enum AnmTimelineError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Parameter error: {0}")]
	AnmParameterError(#[from] AnmParameterError),
	#[error("Unknown timeline command type: {0}")]
	UnknownCommand(u32),
}

impl AnmTimeline {
	pub fn from<R: Read + Seek>(reader: &mut R) -> Result<Self, AnmTimelineError> {
		let raw_command = reader.read_le::<u32>()?;

		let command_type = AnmCommandType::try_from(raw_command)?;

		let p1 = AnmParameter::from(reader)?;
		let p2 = AnmParameter::from(reader)?;
		let p3 = AnmParameter::from(reader)?;

		reader.seek(SeekFrom::Current(5 * 8))?;

		Ok(Self { command_type, parameters: vec![p1, p2, p3] })
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnmCommandType {
	// literally only show and jump get used
	Show	= 0,	// [ID] [min] (max)
	Set		= 1,	// [var] [min] (max)
	Loop	= 2,	// [var] [label]
	Jump	= 3,	// [label]
	If		= 4,	// [var] [label]
	IfE		= 5,	// [var] [value] [label]
	IfN		= 6,	// [var] [value] [label]
	IfG		= 7,	// [var] [value] [label]
	IfS		= 8,	// [var] [value] [label]
	IfGE	= 9,	// [var] [value] [label]
	IfSE	= 10,	// [var] [value] [label]
	Max		= 11,	// [var]
	Blend	= 12,	// [value]
	Disp	= 13,	// [value]
	Pos		= 14,	// [x] [y]
	Wait	= 15,	// [min] (max)
	Add		= 16,	// [var] [value]
	Sub		= 17,	// [var] [value]
}

impl AnmCommandType {
	pub fn as_str(&self) -> &'static str {
		match &self {
			AnmCommandType::Show => "",
			AnmCommandType::Set => "set",
			AnmCommandType::Loop => "loop",
			AnmCommandType::Jump => "jump",
			AnmCommandType::If => "if",
			AnmCommandType::IfE => "ife",
			AnmCommandType::IfN => "ifn",
			AnmCommandType::IfG => "ifg",
			AnmCommandType::IfS => "ifs",
			AnmCommandType::IfGE => "ifge",
			AnmCommandType::IfSE => "ifse",
			AnmCommandType::Max => "max",
			AnmCommandType::Blend => "blend",
			AnmCommandType::Disp => "disp",
			AnmCommandType::Pos => "pos",
			AnmCommandType::Wait => "wait",
			AnmCommandType::Add => "add",
			AnmCommandType::Sub => "sub",
		}
	}
}

impl TryFrom<u32> for AnmCommandType {
    type Error = AnmTimelineError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0  => Ok(Self::Show),
            1  => Ok(Self::Set),
            2  => Ok(Self::Loop),
            3  => Ok(Self::Jump),
            4  => Ok(Self::If),
            5  => Ok(Self::IfE),
            6  => Ok(Self::IfN),
            7  => Ok(Self::IfG),
            8  => Ok(Self::IfS),
            9  => Ok(Self::IfGE),
            10 => Ok(Self::IfSE),
            11 => Ok(Self::Max),
            12 => Ok(Self::Blend),
            13 => Ok(Self::Disp),
            14 => Ok(Self::Pos),
            15 => Ok(Self::Wait),
            16 => Ok(Self::Add),
            17 => Ok(Self::Sub),
            other => Err(AnmTimelineError::UnknownCommand(other)),
        }
    }
}

pub struct AnmParameter {
	pub parameter_type: AnmParameterType,
	pub value: u32,
}

#[derive(Error, Debug)]
pub enum AnmParameterError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Unknown parameter type: {0}")]
	UnknownParameterType(u32),
}

impl AnmParameter {
	pub fn from<R: Read>(reader: &mut R) -> Result<Self, AnmParameterError> {
		let raw_type = reader.read_le::<u32>()?;
		let value = reader.read_le::<u32>()?;

		let parameter_type = AnmParameterType::try_from(raw_type)?;

		Ok(Self { parameter_type, value })
	}
}

impl fmt::Display for AnmParameter {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}={}", self.parameter_type, self.value)
	}
}

impl fmt::Debug for AnmParameter {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}={}", self.parameter_type, self.value)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnmParameterType {
	Number		= 0,
	Variable	= 1,
	Label		= 2,
}

impl TryFrom<u32> for AnmParameterType {
	type Error = AnmParameterError;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match value {
			0 		=> Ok(Self::Number),
			1 		=> Ok(Self::Variable),
			2 		=> Ok(Self::Label),
			other 	=> Err(AnmParameterError::UnknownParameterType(other)),
		}
	}
}
