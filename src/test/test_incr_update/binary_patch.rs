use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct BinaryPatchChunk {
    pub offset: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BinaryPatch {
    pub chunks: Vec<BinaryPatchChunk>,
}

impl BinaryPatch {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(anyhow!("patch bytes too short"));
        }
        let chunk_count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let mut cursor = 4;
        let mut chunks = Vec::with_capacity(chunk_count);
        for _ in 0..chunk_count {
            if cursor + 12 > bytes.len() {
                return Err(anyhow!("unexpected end of patch stream"));
            }
            let offset = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
            cursor += 8;
            let len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + len > bytes.len() {
                return Err(anyhow!("patch payload truncated"));
            }
            let data = bytes[cursor..cursor + len].to_vec();
            cursor += len;
            chunks.push(BinaryPatchChunk { offset, data });
        }
        if cursor != bytes.len() {
            return Err(anyhow!(
                "patch contains {} trailing bytes",
                bytes.len() - cursor
            ));
        }
        Ok(Self { chunks })
    }

    pub fn apply_to_bytes(&self, base: &[u8]) -> Vec<u8> {
        let mut output = base.to_vec();
        for chunk in &self.chunks {
            let start = usize::try_from(chunk.offset).expect("chunk offset exceeds usize");
            let required_len = start.saturating_add(chunk.data.len());
            if output.len() < required_len {
                output.resize(required_len, 0);
            }
            output[start..start + chunk.data.len()].copy_from_slice(&chunk.data);
        }
        output
    }
}
