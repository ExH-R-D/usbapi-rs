use super::configuration::Configuration;
use super::device::Device;
use super::endpoint::Endpoint;
use super::interface::Interface;
use std::io::BufReader;
use std::io::{Bytes, Read};
#[derive(Debug)]
pub struct Descriptor {
    pub descriptor: Vec<u8>,
}

#[derive(Debug)]
pub enum DescriptorType {
    Device(Device),
    Configuration(Configuration),
    String(String),
    Interface(Interface),
    Endpoint(Endpoint),
    ClassSpecific,
    Hub,
    SsEndpointCompanion,
    Unknown(Vec<u8>),
}

/*
impl From<u8> for DescriptorType {
    fn from(original: u8) -> DescriptorType {
        match original {
            1 => DescriptorType::Device,
            2 => DescriptorType::Configuration,
            3 => DescriptorType::String,
            4 => DescriptorType::Interface,
            5 => DescriptorType::Endpoint,
            0x24 => DescriptorType::ClassSpecific,
            0x29 => DescriptorType::Hub,
            0x30 => DescriptorType::SsEndpointCompanion,
            _n => DescriptorType::Unknown
        }
    }
}
*/

// FIXME would probably be better return an enum
// Something like DescriptorType::XXX(yyy) where yyy is a struct of type
impl Iterator for Descriptor {
    type Item = DescriptorType;
    fn next(&mut self) -> Option<DescriptorType> {
        if self.descriptor.is_empty() {
            // We are done
            return None;
        }

        let dlength = self.descriptor[0] as usize;
        if dlength > self.descriptor.len() || dlength == 2 {
            eprintln!("Invalid descriptor field > vec.len() bailout");
            return None;
        }

        let kind = self.descriptor[1];
        let mut iter = self.descriptor.iter();
        let res: DescriptorType = match kind {
            1 => DescriptorType::Device(Device::new(&mut iter)?),
            2 => DescriptorType::Configuration(Configuration::new(&mut iter)?),
            3 => DescriptorType::String("FIXME handle string type".to_string()),
            4 => DescriptorType::Interface(Interface::new(&mut iter)?),
            5 => DescriptorType::Endpoint(Endpoint::new(&mut iter)?),
            _ => {
                log::debug!("Found unknown descriptor: {} {}", kind, dlength);
                let res = DescriptorType::Unknown(self.descriptor[..dlength].to_vec());
                //                if dlength == 0 {
                self.descriptor = vec![];
                //               }
                return Some(res);
            }
        };
        self.descriptor = self.descriptor[dlength..].to_vec();

        Some(res)
    }
}

impl Descriptor {
    pub fn from_buf_reader(reader: &mut BufReader<&std::fs::File>) -> Self {
        let mut desc = Descriptor { descriptor: vec![] };
        if let Err(err) = reader.read_to_end(&mut desc.descriptor) {
            println!("{}", err);
        }

        desc
    }

    /// FIXME uglyish hackish kill it
    pub fn from_path(file_path: &std::path::Path) -> Option<Self> {
        use std::fs::File;
        let file = match File::open(file_path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}", e);
                return None;
            }
        };
        match Self::from_bytes(file.bytes()) {
            Ok(d) => Some(d),
            Err(_) => None,
        }
    }

    pub fn from_bytes<T>(bytes: Bytes<T>) -> Result<Self, std::io::Error>
    where
        T: Read,
    {
        let mut desc = Descriptor { descriptor: vec![] };
        for byte in bytes {
            desc.descriptor.push(byte.unwrap());
        }

        Ok(desc)
    }
}
