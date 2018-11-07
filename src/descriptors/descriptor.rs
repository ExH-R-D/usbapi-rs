pub struct Descriptor {
    pub descriptor: Vec<u8>
}

#[derive(Debug)]
pub enum DescriptorType {
    Device,
    Configuration,
    String,
    Interface,
    Endpoint,
    ClassSpecific,
    Hub,
    SsEndpointCompanion,
    Unknown,
}

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
            n => DescriptorType::Unknown
        }
    }
}

impl Iterator for Descriptor {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Vec<u8>> {
        if self.descriptor.len() < 1 {
            return None
        }

        let dlength = self.descriptor[0] as usize;
        let give = self.descriptor[..dlength].to_vec();
        self.descriptor = self.descriptor[dlength..].to_vec();
        Some(give)
    }
}

// Just a used trait when create below
// Descriptors...
impl Descriptor {
    pub fn new(data: Vec<u8>) -> Self {
        Descriptor { descriptor: data }
    }
}

