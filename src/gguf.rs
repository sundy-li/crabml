use std::{io::Read, collections::HashMap};

const GGUF_MAGIC: u64 = 0x46554747;
const GGUF_VERSION: u64 = 2;
const GGUF_DEFAULT_ALIGNMENT: u64 = 32;

// General
const KEY_GENERAL_ARCHITECTURE: &str = "general.architecture";
const KEY_GENERAL_QUANTIZATION_VERSION: &str = "general.quantization_version";
const KEY_GENERAL_ALIGNMENT: &str = "general.alignment";
const KEY_GENERAL_NAME: &str = "general.name";
const KEY_GENERAL_AUTHOR: &str = "general.author";
const KEY_GENERAL_URL: &str = "general.url";
const KEY_GENERAL_DESCRIPTION: &str = "general.description";
const KEY_GENERAL_LICENSE: &str = "general.license";
const KEY_GENERAL_SOURCE_URL: &str = "general.source.url";
const KEY_GENERAL_SOURCE_HF_REPO: &str = "general.source.hugginface.repository";
const KEY_GENERAL_FILE_TYPE: &str = "general.file_type";

// LLM
const KEY_CONTEXT_LENGTH: &str = "{arch}.context_length";
const KEY_EMBEDDING_LENGTH: &str = "{arch}.embedding_length";
const KEY_BLOCK_COUNT: &str = "{arch}.block_count";
const KEY_FEED_FORWARD_LENGTH: &str = "{arch}.feed_forward_length";
const KEY_USE_PARALLEL_RESIDUAL: &str = "{arch}.use_parallel_residual";
const KEY_TENSOR_DATA_LAYOUT: &str = "{arch}.tensor_data_layout";

// Attention
const KEY_ATTENTION_HEAD_COUNT: &str = "{arch}.attention.head_count";
const KEY_ATTENTION_HEAD_COUNT_KV: &str = "{arch}.attention.head_count_kv";
const KEY_ATTENTION_MAX_ALIBI_BIAS: &str = "{arch}.attention.max_alibi_bias";
const KEY_ATTENTION_CLAMP_KQV: &str = "{arch}.attention.clamp_kqv";
const KEY_ATTENTION_LAYERNORM_EPS: &str = "{arch}.attention.layer_norm_epsilon";
const KEY_ATTENTION_LAYERNORM_RMS_EPS: &str = "{arch}.attention.layer_norm_rms_epsilon";

// RoPE
const KEY_ROPE_DIMENSION_COUNT: &str = "{arch}.rope.dimension_count";
const KEY_ROPE_FREQ_BASE: &str = "{arch}.rope.freq_base";
const KEY_ROPE_SCALE_LINEAR: &str = "{arch}.rope.scale_linear";

// Tokenization
const KEY_TOKENIZER_MODEL: &str = "tokenizer.ggml.model";
const KEY_TOKENIZER_LIST: &str = "tokenizer.ggml.tokens";
const KEY_TOKENIZER_TOKEN_TYPE: &str = "tokenizer.ggml.token_type";
const KEY_TOKENIZER_SCORES: &str = "tokenizer.ggml.scores";
const KEY_TOKENIZER_MERGES: &str = "tokenizer.ggml.merges";
const KEY_TOKENIZER_BOS_ID: &str = "tokenizer.ggml.bos_token_id";
const KEY_TOKENIZER_EOS_ID: &str = "tokenizer.ggml.eos_token_id";
const KEY_TOKENIZER_UNK_ID: &str = "tokenizer.ggml.unknown_token_id";
const KEY_TOKENIZER_SEP_ID: &str = "tokenizer.ggml.seperator_token_id";
const KEY_TOKENIZER_PAD_ID: &str = "tokenizer.ggml.padding_token_id";
const KEY_TOKENIZER_HF_JSON: &str = "tokenizer.huggingface.json";
const KEY_TOKENIZER_RWKV: &str = "tokenizer.rwkv.world";

#[derive(Debug)]
pub enum ModelArch {
    Llama = 0,
    Falcon = 1,
    GPT2 = 2,
    GPTJ = 3,
    GPTNEOX = 4,
    MPT = 5,
}

#[derive(Debug)]
pub enum ModelTensor {
    TOKEN_EMBD = 0,
    POS_EMBD = 1,
    OUTPUT = 2,
    OUTPUT_NORM = 3,
    ROPE_FREQS = 4,
    ATTN_Q = 5,
    ATTN_K = 6,
    ATTN_V = 7,
    ATTN_QKV = 8,
    ATTN_OUT = 9,
    ATTN_NORM = 10,
    ATTN_NORM_2 = 11,
    ATTN_ROT_EMBD = 12,
    FFN_GATE = 13,
    FFN_DOWN = 14,
    FFN_UP = 15,
    FFN_NORM = 16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GGUFValueType {
    // The value is a 8-bit unsigned integer.
    U8 = 0,
    // The value is a 8-bit signed little-endian integer.
    I8 = 1,
    // The value is a 16-bit unsigned little-endian integer.
    U16 = 2,
    // The value is a 16-bit signed little-endian integer.
    I16 = 3,
    // The value is a 32-bit unsigned little-endian integer.
    U32 = 4,
    // The value is a 32-bit signed little-endian integer.
    I32 = 5,
    // The value is a 32-bit IEEE754 floating point number.
    F32 = 6,
    // The value is a boolean.
    // 1-byte value where 0 is false and 1 is true.
    // Anything else is invalid, and should be treated as either the model being invalid or the reader being buggy.
    Bool = 7,
    // The value is a UTF-8 non-null-terminated string, with length prepended.
    String = 8,
    // The value is an array of other values, with the length and type prepended.
    ///
    // Arrays can be nested, and the length of the array is the number of elements in the array, not the number of bytes.
    Array = 9,
    // The value is a 64-bit unsigned little-endian integer.
    U64 = 10,
    // The value is a 64-bit signed little-endian integer.
    I64 = 11,
    // The value is a 64-bit IEEE754 floating point number.
    F64 = 12,
}

#[derive(Debug, Clone)]
pub enum GGUFValue<'a> {
    U8(u8),
    U8Array(&'a [u8]),
    I8(i8),
    I8Array(&'a [i8]),
    U16(u16),
    U16Array(&'a [u16]),
    I16(i16),
    I16Array(&'a [i16]),
    U32(u32),
    U32Array(&'a [u32]),
    I32(i32),
    I32Array(&'a [i32]),
    U64(u64),
    U64Array(&'a [u64]),
    I64(i64),
    I64Array(&'a [i64]),
    F32(f32),
    F32Array(&'a [f32]),
    F64(f64),
    F64Array(&'a [f64]),
    Bool(u8),
    BoolArray(&'a [u8]),
    String(&'a str),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum GGUFReaderErrorKind {
    Unexpected,
    DataError,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct GGUFError {
    kind: GGUFReaderErrorKind,
    msg: String,
}

pub type Result<T> = std::result::Result<T, GGUFError>;

pub struct GGUFHeader {
    // Magic number to announce that this is a GGUF file.
    // Must be `GGUF` at the byte level: `0x47` `0x47` `0x55` `0x46`.
    // Your executor might do little-endian byte order, so it might be
    // check for 0x46554747 and letting the endianness cancel out.
    // Consider being *very* explicit about the byte order here.
    magic: u32,
    // The version of the format implemented.
    // Must be `2` for version described in this spec.
    //
    // This version should only be increased for structural changes to the format.
    // Changes that do not affect the structure of the file should instead update the metadata
    // to signify the change.
    version: u32,
    // The number of tensors in the file.
    // This is explicit, instead of being included in the metadata, to ensure it is always present
    // for loading the tensors.
    tensor_count: u64,
    // The number of metadata key-value pairs.
    metadata_kv: HashMap<String, GGUFValue>,
}

pub struct GGUFReader<R: Read> {
    r: R,
    arch: String,
}

impl<R> GGUFReader<R>
where
    R: Read,
{
    fn read_value(&mut self) -> Result<GGUFValue> {
        todo!()
    }
}