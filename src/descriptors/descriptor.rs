use std::io::Read;
use std::io::BufReader;
use super::device::Device;
use super::configuration::Configuration;
use super::interface::Interface;
use super::endpoint::Endpoint;
pub struct Descriptor {
    pub descriptor: Vec<u8>
}

//#[derive(Debug)]
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
        if self.descriptor.len() == 0 {
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
            3 => DescriptorType::String("FIXME".to_string()),
            4 => DescriptorType::Interface(Interface::new(&mut iter)?),
            5 => DescriptorType::Endpoint(Endpoint::new(&mut iter)?),
            _ => DescriptorType::Unknown(self.descriptor[..dlength].to_vec())
        };
        self.descriptor = self.descriptor[dlength..].to_vec();
        
        Some(res)
    }
}

impl Descriptor {
    pub fn from_buf_reader(reader: &mut BufReader<&std::fs::File>) -> Self {
        let mut desc = Descriptor { descriptor: vec!() };
        if let Err(err) = reader.read_to_end(&mut desc.descriptor) {
            println!("{}", err);
        }

        desc
    }
}

