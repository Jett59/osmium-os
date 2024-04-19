use crate::SyscallResultDecodeError;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogArguments {
    pub string_address: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogError {
    InvalidUtf8 { position: usize },
}

const PARAMETER_COUNT: usize = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct EncodedResult {
    status_code: usize,
    parameters: [usize; PARAMETER_COUNT],
}

const INVALID_UTF8_STATUS_CODE: usize = 1;

#[inline]
pub(crate) fn encode_log_result(result: Result<(), LogError>) -> EncodedResult {
    match result {
        Ok(()) => EncodedResult {
            status_code: 0,
            parameters: [0],
        },
        Err(LogError::InvalidUtf8 { position }) => EncodedResult {
            status_code: INVALID_UTF8_STATUS_CODE,
            parameters: [position],
        },
    }
}

#[inline]
pub(crate) fn decode_log_result(
    result: EncodedResult,
) -> Result<Result<(), LogError>, SyscallResultDecodeError> {
    match (result.status_code, result.parameters) {
        (0, _) => Ok(Ok(())),
        (INVALID_UTF8_STATUS_CODE, [position]) => Ok(Err(LogError::InvalidUtf8 { position })),
        _ => Err(SyscallResultDecodeError::InvalidResultField("status_code")),
    }
}
