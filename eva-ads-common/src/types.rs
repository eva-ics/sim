use binrw::{prelude::*, BinRead, BinWrite};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::io::{Cursor, Write};

macro_rules! impl_err {
    ($err: ty, $tgt: expr) => {
        impl From<$err> for AdsError {
            fn from(_err: $err) -> Self {
                $tgt
            }
        }
    };
}

impl_err!(std::num::TryFromIntError, AdsError::InternalError);
impl_err!(Box<dyn std::error::Error>, AdsError::InternalError);
impl_err!(binrw::Error, AdsError::InternalError);
impl_err!(std::io::Error, AdsError::NoIo);

impl std::error::Error for AdsError {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum AdsError {
    InternalError = 0x001,
    NoRealTime = 0x002,
    AllocationLockedMemoryError = 0x003,
    MailboxFullAdsMessageCouldNotBeSent = 0x004,
    WrongReceiveHmsg = 0x005,
    TargetPortNotFoundPossiblyAdsServerNotStarted = 0x006,
    TargetMachineNotFoundPossiblyMissingAdsRoutes = 0x007,
    UnknownCommandId = 0x008,
    InvalidTaskId = 0x009,
    NoIo = 0x00A,
    UnknownAmsCommand = 0x00B,
    Win32Error = 0x00C,
    PortNotConnected = 0x00D,
    InvalidAmsLength = 0x00E,
    InvalidAmsNetID = 0x00F,
    LowInstallationLevel = 0x010,
    NoDebuggingAvailable = 0x011,
    PortDisabledSystemServiceNotStarted = 0x012,
    PortAlreadyConnected = 0x013,
    AmsSyncWin32Error = 0x014,
    AmsSyncTimeout = 0x015,
    AmsSyncError = 0x016,
    AmsSyncNoIndexMap = 0x017,
    InvalidAmsPort = 0x018,
    NoMemory = 0x019,
    TcpSendError = 0x01A,
    HostUnreachable = 0x01B,
    InvalidAmsFragment = 0x01C,
    TlsSendErrorSecureAdsConnectionFailed = 0x01D,
    AccessDeniedSecureAdsAccessDenied = 0x01E,
    RouterNoLockedMemory = 0x500,
    RouterMemorySizeCouldNotBeChanged = 0x501,
    RouterMailboxFull = 0x502,
    RouterDebugMailboxFull = 0x503,
    RouterPortTypeIsUnknown = 0x504,
    RouterIsNotInitialized = 0x505,
    RouterDesiredPortNumberIsAlreadyAssigned = 0x506,
    RouterPortNotRegistered = 0x507,
    RouterMaximumNumberOfPortsReached = 0x508,
    RouterPortIsInvalid = 0x509,
    RouterIsNotActive = 0x50A,
    RouterMailboxFullForFragmentedMessages = 0x50B,
    RouterFragmentTimeoutOccurred = 0x50C,
    RouterPortRemoved = 0x50D,
    GeneralDeviceError = 0x700,
    ServiceIsNotSupportedByServer = 0x701,
    InvalidIndexGroup = 0x702,
    InvalidIndexOffset = 0x703,
    ReadingWritingNotPermitted = 0x704,
    ParameterSizeNotCorrect = 0x705,
    InvalidParameterValueS = 0x706,
    DeviceIsNotInAReadyState = 0x707,
    DeviceIsBusy = 0x708,
    InvalidOsContextUseMultiTaskDataAccess = 0x709,
    OutOfMemory = 0x70A,
    InvalidParameterValueS1 = 0x70B,
    NotFoundFiles = 0x70C,
    SyntaxErrorInCommandOrFile = 0x70D,
    ObjectsDoNotMatch = 0x70E,
    ObjectAlreadyExists = 0x70F,
    SymbolNotFound = 0x710,
    SymbolVersionInvalidCreateANewHandle = 0x711,
    ServerIsInAnInvalidState = 0x712,
    AdsTransModeNotSupported = 0x713,
    NotificationHandleIsInvalid = 0x714,
    NotificationClientNotRegistered = 0x715,
    NoMoreNotificationHandles = 0x716,
    NotificationSizeTooLarge = 0x717,
    DeviceNotInitialized = 0x718,
    DeviceHasATimeout = 0x719,
    QueryInterfaceFailed = 0x71A,
    WrongInterfaceRequired = 0x71B,
    ClassIdIsInvalid = 0x71C,
    ObjectIdIsInvalid = 0x71D,
    RequestIsPending = 0x71E,
    RequestIsAborted = 0x71F,
    SignalWarning = 0x720,
    InvalidArrayIndex = 0x721,
    SymbolNotActiveReleaseHandleAndTryAgain = 0x722,
    AccessDenied = 0x723,
    NoLicenseFoundActivateLicense = 0x724,
    LicenseExpired = 0x725,
    LicenseExceeded = 0x726,
    LicenseInvalid = 0x727,
    InvalidSystemIdInLicense = 0x728,
    LicenseNotTimeLimited = 0x729,
    LicenseIssueTimeInTheFuture = 0x72A,
    LicenseTimePeriodTooLong = 0x72B,
    ExceptionInDeviceSpecificCodeCheckEachDevice = 0x72C,
    LicenseFileReadTwice = 0x72D,
    InvalidSignature = 0x72E,
    InvalidPublicKeyCertificate = 0x72F,
    PublicKeyNotKnownFromOem = 0x730,
    LicenseNotValidForThisSystemId = 0x731,
    DemoLicenseProhibited = 0x732,
    InvalidFunctionId = 0x733,
    OutsideTheValidRange = 0x734,
    InvalidAlignment = 0x735,
    InvalidPlatformLevel = 0x736,
    ContextForwardToPassiveLevel = 0x737,
    ContentForwardToDispatchLevel = 0x738,
    ContextForwardToRealTime = 0x739,
    GeneralClientError = 0x740,
    InvalidParameterAtService = 0x741,
    PollingListIsEmpty = 0x742,
    VarConnectionAlreadyInUse = 0x743,
    InvokeIdInUse = 0x744,
    TimeoutElapsedCheckRouteSetting = 0x745,
    ErrorInWin32Subsystem = 0x746,
    InvalidClientTimeoutValue = 0x747,
    AdsPortNotOpened = 0x748,
    NoAmsAddress = 0x749,
    InternalErrorInAdsSync = 0x750,
    HashTableOverflow = 0x751,
    KeyNotFoundInHash = 0x752,
    NoMoreSymbolsInCache = 0x753,
    InvalidResponseReceived = 0x754,
    SyncPortIsLocked = 0x755,
    InternalErrorInRealTimeSystem = 0x1000,
    TimerValueNotValid = 0x1001,
    TaskPointerHasInvalidValue0 = 0x1002,
    StackPointerHasInvalidValue0 = 0x1003,
    RequestedTaskPriorityAlreadyAssigned = 0x1004,
    NoFreeTaskControlBlock = 0x1005,
    NoFreeSemaphores = 0x1006,
    NoFreeSpaceInTheQueue = 0x1007,
    ExternalSyncInterruptAlreadyApplied = 0x100D,
    NoExternalSyncInterruptApplied = 0x100E,
    ExternalSyncInterruptApplicationFailed = 0x100F,
    CallOfServiceFunctionInWrongContext = 0x1010,
    IntelVtXNotSupported = 0x1017,
    IntelVtXNotEnabledInBios = 0x1018,
    MissingFunctionInIntelVtX = 0x1019,
    ActivationOfIntelVtXFailed = 0x101A,
    Unknown = 0xFFFF_FFFF,
}

impl From<u32> for AdsError {
    #[allow(clippy::too_many_lines)]
    fn from(v: u32) -> Self {
        match v {
            x if x == AdsError::InternalError as u32 => AdsError::InternalError,
            x if x == AdsError::NoRealTime as u32 => AdsError::NoRealTime,
            x if x == AdsError::AllocationLockedMemoryError as u32 => {
                AdsError::AllocationLockedMemoryError
            }
            x if x == AdsError::MailboxFullAdsMessageCouldNotBeSent as u32 => {
                AdsError::MailboxFullAdsMessageCouldNotBeSent
            }
            x if x == AdsError::WrongReceiveHmsg as u32 => AdsError::WrongReceiveHmsg,
            x if x == AdsError::TargetPortNotFoundPossiblyAdsServerNotStarted as u32 => {
                AdsError::TargetPortNotFoundPossiblyAdsServerNotStarted
            }
            x if x == AdsError::TargetMachineNotFoundPossiblyMissingAdsRoutes as u32 => {
                AdsError::TargetMachineNotFoundPossiblyMissingAdsRoutes
            }
            x if x == AdsError::UnknownCommandId as u32 => AdsError::UnknownCommandId,
            x if x == AdsError::InvalidTaskId as u32 => AdsError::InvalidTaskId,
            x if x == AdsError::NoIo as u32 => AdsError::NoIo,
            x if x == AdsError::UnknownAmsCommand as u32 => AdsError::UnknownAmsCommand,
            x if x == AdsError::Win32Error as u32 => AdsError::Win32Error,
            x if x == AdsError::PortNotConnected as u32 => AdsError::PortNotConnected,
            x if x == AdsError::InvalidAmsLength as u32 => AdsError::InvalidAmsLength,
            x if x == AdsError::InvalidAmsNetID as u32 => AdsError::InvalidAmsNetID,
            x if x == AdsError::LowInstallationLevel as u32 => AdsError::LowInstallationLevel,
            x if x == AdsError::NoDebuggingAvailable as u32 => AdsError::NoDebuggingAvailable,
            x if x == AdsError::PortDisabledSystemServiceNotStarted as u32 => {
                AdsError::PortDisabledSystemServiceNotStarted
            }
            x if x == AdsError::PortAlreadyConnected as u32 => AdsError::PortAlreadyConnected,
            x if x == AdsError::AmsSyncWin32Error as u32 => AdsError::AmsSyncWin32Error,
            x if x == AdsError::AmsSyncTimeout as u32 => AdsError::AmsSyncTimeout,
            x if x == AdsError::AmsSyncError as u32 => AdsError::AmsSyncError,
            x if x == AdsError::AmsSyncNoIndexMap as u32 => AdsError::AmsSyncNoIndexMap,
            x if x == AdsError::InvalidAmsPort as u32 => AdsError::InvalidAmsPort,
            x if x == AdsError::NoMemory as u32 => AdsError::NoMemory,
            x if x == AdsError::TcpSendError as u32 => AdsError::TcpSendError,
            x if x == AdsError::HostUnreachable as u32 => AdsError::HostUnreachable,
            x if x == AdsError::InvalidAmsFragment as u32 => AdsError::InvalidAmsFragment,
            x if x == AdsError::TlsSendErrorSecureAdsConnectionFailed as u32 => {
                AdsError::TlsSendErrorSecureAdsConnectionFailed
            }
            x if x == AdsError::AccessDeniedSecureAdsAccessDenied as u32 => {
                AdsError::AccessDeniedSecureAdsAccessDenied
            }
            x if x == AdsError::RouterNoLockedMemory as u32 => AdsError::RouterNoLockedMemory,
            x if x == AdsError::RouterMemorySizeCouldNotBeChanged as u32 => {
                AdsError::RouterMemorySizeCouldNotBeChanged
            }
            x if x == AdsError::RouterMailboxFull as u32 => AdsError::RouterMailboxFull,
            x if x == AdsError::RouterDebugMailboxFull as u32 => AdsError::RouterDebugMailboxFull,
            x if x == AdsError::RouterPortTypeIsUnknown as u32 => AdsError::RouterPortTypeIsUnknown,
            x if x == AdsError::RouterIsNotInitialized as u32 => AdsError::RouterIsNotInitialized,
            x if x == AdsError::RouterDesiredPortNumberIsAlreadyAssigned as u32 => {
                AdsError::RouterDesiredPortNumberIsAlreadyAssigned
            }
            x if x == AdsError::RouterPortNotRegistered as u32 => AdsError::RouterPortNotRegistered,
            x if x == AdsError::RouterMaximumNumberOfPortsReached as u32 => {
                AdsError::RouterMaximumNumberOfPortsReached
            }
            x if x == AdsError::RouterPortIsInvalid as u32 => AdsError::RouterPortIsInvalid,
            x if x == AdsError::RouterIsNotActive as u32 => AdsError::RouterIsNotActive,
            x if x == AdsError::RouterMailboxFullForFragmentedMessages as u32 => {
                AdsError::RouterMailboxFullForFragmentedMessages
            }
            x if x == AdsError::RouterFragmentTimeoutOccurred as u32 => {
                AdsError::RouterFragmentTimeoutOccurred
            }
            x if x == AdsError::RouterPortRemoved as u32 => AdsError::RouterPortRemoved,
            x if x == AdsError::GeneralDeviceError as u32 => AdsError::GeneralDeviceError,
            x if x == AdsError::ServiceIsNotSupportedByServer as u32 => {
                AdsError::ServiceIsNotSupportedByServer
            }
            x if x == AdsError::InvalidIndexGroup as u32 => AdsError::InvalidIndexGroup,
            x if x == AdsError::InvalidIndexOffset as u32 => AdsError::InvalidIndexOffset,
            x if x == AdsError::ReadingWritingNotPermitted as u32 => {
                AdsError::ReadingWritingNotPermitted
            }
            x if x == AdsError::ParameterSizeNotCorrect as u32 => AdsError::ParameterSizeNotCorrect,
            x if x == AdsError::InvalidParameterValueS as u32 => AdsError::InvalidParameterValueS,
            x if x == AdsError::DeviceIsNotInAReadyState as u32 => {
                AdsError::DeviceIsNotInAReadyState
            }
            x if x == AdsError::DeviceIsBusy as u32 => AdsError::DeviceIsBusy,
            x if x == AdsError::InvalidOsContextUseMultiTaskDataAccess as u32 => {
                AdsError::InvalidOsContextUseMultiTaskDataAccess
            }
            x if x == AdsError::OutOfMemory as u32 => AdsError::OutOfMemory,
            x if x == AdsError::InvalidParameterValueS1 as u32 => AdsError::InvalidParameterValueS1,
            x if x == AdsError::NotFoundFiles as u32 => AdsError::NotFoundFiles,
            x if x == AdsError::SyntaxErrorInCommandOrFile as u32 => {
                AdsError::SyntaxErrorInCommandOrFile
            }
            x if x == AdsError::ObjectsDoNotMatch as u32 => AdsError::ObjectsDoNotMatch,
            x if x == AdsError::ObjectAlreadyExists as u32 => AdsError::ObjectAlreadyExists,
            x if x == AdsError::SymbolNotFound as u32 => AdsError::SymbolNotFound,
            x if x == AdsError::SymbolVersionInvalidCreateANewHandle as u32 => {
                AdsError::SymbolVersionInvalidCreateANewHandle
            }
            x if x == AdsError::ServerIsInAnInvalidState as u32 => {
                AdsError::ServerIsInAnInvalidState
            }
            x if x == AdsError::AdsTransModeNotSupported as u32 => {
                AdsError::AdsTransModeNotSupported
            }
            x if x == AdsError::NotificationHandleIsInvalid as u32 => {
                AdsError::NotificationHandleIsInvalid
            }
            x if x == AdsError::NotificationClientNotRegistered as u32 => {
                AdsError::NotificationClientNotRegistered
            }
            x if x == AdsError::NoMoreNotificationHandles as u32 => {
                AdsError::NoMoreNotificationHandles
            }
            x if x == AdsError::NotificationSizeTooLarge as u32 => {
                AdsError::NotificationSizeTooLarge
            }
            x if x == AdsError::DeviceNotInitialized as u32 => AdsError::DeviceNotInitialized,
            x if x == AdsError::DeviceHasATimeout as u32 => AdsError::DeviceHasATimeout,
            x if x == AdsError::QueryInterfaceFailed as u32 => AdsError::QueryInterfaceFailed,
            x if x == AdsError::WrongInterfaceRequired as u32 => AdsError::WrongInterfaceRequired,
            x if x == AdsError::ClassIdIsInvalid as u32 => AdsError::ClassIdIsInvalid,
            x if x == AdsError::ObjectIdIsInvalid as u32 => AdsError::ObjectIdIsInvalid,
            x if x == AdsError::RequestIsPending as u32 => AdsError::RequestIsPending,
            x if x == AdsError::RequestIsAborted as u32 => AdsError::RequestIsAborted,
            x if x == AdsError::SignalWarning as u32 => AdsError::SignalWarning,
            x if x == AdsError::InvalidArrayIndex as u32 => AdsError::InvalidArrayIndex,
            x if x == AdsError::SymbolNotActiveReleaseHandleAndTryAgain as u32 => {
                AdsError::SymbolNotActiveReleaseHandleAndTryAgain
            }
            x if x == AdsError::AccessDenied as u32 => AdsError::AccessDenied,
            x if x == AdsError::NoLicenseFoundActivateLicense as u32 => {
                AdsError::NoLicenseFoundActivateLicense
            }
            x if x == AdsError::LicenseExpired as u32 => AdsError::LicenseExpired,
            x if x == AdsError::LicenseExceeded as u32 => AdsError::LicenseExceeded,
            x if x == AdsError::LicenseInvalid as u32 => AdsError::LicenseInvalid,
            x if x == AdsError::InvalidSystemIdInLicense as u32 => {
                AdsError::InvalidSystemIdInLicense
            }
            x if x == AdsError::LicenseNotTimeLimited as u32 => AdsError::LicenseNotTimeLimited,
            x if x == AdsError::LicenseIssueTimeInTheFuture as u32 => {
                AdsError::LicenseIssueTimeInTheFuture
            }
            x if x == AdsError::LicenseTimePeriodTooLong as u32 => {
                AdsError::LicenseTimePeriodTooLong
            }
            x if x == AdsError::ExceptionInDeviceSpecificCodeCheckEachDevice as u32 => {
                AdsError::ExceptionInDeviceSpecificCodeCheckEachDevice
            }
            x if x == AdsError::LicenseFileReadTwice as u32 => AdsError::LicenseFileReadTwice,
            x if x == AdsError::InvalidSignature as u32 => AdsError::InvalidSignature,
            x if x == AdsError::InvalidPublicKeyCertificate as u32 => {
                AdsError::InvalidPublicKeyCertificate
            }
            x if x == AdsError::PublicKeyNotKnownFromOem as u32 => {
                AdsError::PublicKeyNotKnownFromOem
            }
            x if x == AdsError::LicenseNotValidForThisSystemId as u32 => {
                AdsError::LicenseNotValidForThisSystemId
            }
            x if x == AdsError::DemoLicenseProhibited as u32 => AdsError::DemoLicenseProhibited,
            x if x == AdsError::InvalidFunctionId as u32 => AdsError::InvalidFunctionId,
            x if x == AdsError::OutsideTheValidRange as u32 => AdsError::OutsideTheValidRange,
            x if x == AdsError::InvalidAlignment as u32 => AdsError::InvalidAlignment,
            x if x == AdsError::InvalidPlatformLevel as u32 => AdsError::InvalidPlatformLevel,
            x if x == AdsError::ContextForwardToPassiveLevel as u32 => {
                AdsError::ContextForwardToPassiveLevel
            }
            x if x == AdsError::ContentForwardToDispatchLevel as u32 => {
                AdsError::ContentForwardToDispatchLevel
            }
            x if x == AdsError::ContextForwardToRealTime as u32 => {
                AdsError::ContextForwardToRealTime
            }
            x if x == AdsError::GeneralClientError as u32 => AdsError::GeneralClientError,
            x if x == AdsError::InvalidParameterAtService as u32 => {
                AdsError::InvalidParameterAtService
            }
            x if x == AdsError::PollingListIsEmpty as u32 => AdsError::PollingListIsEmpty,
            x if x == AdsError::VarConnectionAlreadyInUse as u32 => {
                AdsError::VarConnectionAlreadyInUse
            }
            x if x == AdsError::InvokeIdInUse as u32 => AdsError::InvokeIdInUse,
            x if x == AdsError::TimeoutElapsedCheckRouteSetting as u32 => {
                AdsError::TimeoutElapsedCheckRouteSetting
            }
            x if x == AdsError::ErrorInWin32Subsystem as u32 => AdsError::ErrorInWin32Subsystem,
            x if x == AdsError::InvalidClientTimeoutValue as u32 => {
                AdsError::InvalidClientTimeoutValue
            }
            x if x == AdsError::AdsPortNotOpened as u32 => AdsError::AdsPortNotOpened,
            x if x == AdsError::NoAmsAddress as u32 => AdsError::NoAmsAddress,
            x if x == AdsError::InternalErrorInAdsSync as u32 => AdsError::InternalErrorInAdsSync,
            x if x == AdsError::HashTableOverflow as u32 => AdsError::HashTableOverflow,
            x if x == AdsError::KeyNotFoundInHash as u32 => AdsError::KeyNotFoundInHash,
            x if x == AdsError::NoMoreSymbolsInCache as u32 => AdsError::NoMoreSymbolsInCache,
            x if x == AdsError::InvalidResponseReceived as u32 => AdsError::InvalidResponseReceived,
            x if x == AdsError::SyncPortIsLocked as u32 => AdsError::SyncPortIsLocked,
            x if x == AdsError::InternalErrorInRealTimeSystem as u32 => {
                AdsError::InternalErrorInRealTimeSystem
            }
            x if x == AdsError::TimerValueNotValid as u32 => AdsError::TimerValueNotValid,
            x if x == AdsError::TaskPointerHasInvalidValue0 as u32 => {
                AdsError::TaskPointerHasInvalidValue0
            }
            x if x == AdsError::StackPointerHasInvalidValue0 as u32 => {
                AdsError::StackPointerHasInvalidValue0
            }
            x if x == AdsError::RequestedTaskPriorityAlreadyAssigned as u32 => {
                AdsError::RequestedTaskPriorityAlreadyAssigned
            }
            x if x == AdsError::NoFreeTaskControlBlock as u32 => AdsError::NoFreeTaskControlBlock,
            x if x == AdsError::NoFreeSemaphores as u32 => AdsError::NoFreeSemaphores,
            x if x == AdsError::NoFreeSpaceInTheQueue as u32 => AdsError::NoFreeSpaceInTheQueue,
            x if x == AdsError::ExternalSyncInterruptAlreadyApplied as u32 => {
                AdsError::ExternalSyncInterruptAlreadyApplied
            }
            x if x == AdsError::NoExternalSyncInterruptApplied as u32 => {
                AdsError::NoExternalSyncInterruptApplied
            }
            x if x == AdsError::ExternalSyncInterruptApplicationFailed as u32 => {
                AdsError::ExternalSyncInterruptApplicationFailed
            }
            x if x == AdsError::CallOfServiceFunctionInWrongContext as u32 => {
                AdsError::CallOfServiceFunctionInWrongContext
            }
            x if x == AdsError::IntelVtXNotSupported as u32 => AdsError::IntelVtXNotSupported,
            x if x == AdsError::IntelVtXNotEnabledInBios as u32 => {
                AdsError::IntelVtXNotEnabledInBios
            }
            x if x == AdsError::MissingFunctionInIntelVtX as u32 => {
                AdsError::MissingFunctionInIntelVtX
            }
            x if x == AdsError::ActivationOfIntelVtXFailed as u32 => {
                AdsError::ActivationOfIntelVtXFailed
            }
            _ => AdsError::Unknown,
        }
    }
}

impl From<AdsError> for eva_common::Error {
    fn from(e: AdsError) -> Self {
        match e {
            AdsError::SymbolNotFound
            | AdsError::InvalidIndexGroup
            | AdsError::InvalidIndexOffset => eva_common::Error::not_found(e),
            _ => eva_common::Error::failed(e),
        }
    }
}

impl std::fmt::Display for AdsError {
    #[allow(clippy::too_many_lines, clippy::match_same_arms)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdsError::InternalError => {
                write!(f, "Internal error")?;
            }
            AdsError::NoRealTime => {
                write!(f, "No real-time")?;
            }
            AdsError::AllocationLockedMemoryError => {
                write!(f, "Allocation locked - memory error")?;
            }
            AdsError::MailboxFullAdsMessageCouldNotBeSent => {
                write!(f, "Mailbox full - ADS message could not be sent")?;
            }
            AdsError::WrongReceiveHmsg => {
                write!(f, "Wrong receive HMSG")?;
            }
            AdsError::TargetPortNotFoundPossiblyAdsServerNotStarted => {
                write!(f, "Target port not found,possibly ADS server not started")?;
            }
            AdsError::TargetMachineNotFoundPossiblyMissingAdsRoutes => {
                write!(f, "Target machine not found,possibly missing ADS routes")?;
            }
            AdsError::UnknownCommandId => {
                write!(f, "Unknown command ID")?;
            }
            AdsError::InvalidTaskId => {
                write!(f, "Invalid task ID")?;
            }
            AdsError::NoIo => {
                write!(f, "No IO")?;
            }
            AdsError::UnknownAmsCommand => {
                write!(f, "Unknown AMS command")?;
            }
            AdsError::Win32Error => {
                write!(f, "Win32 error")?;
            }
            AdsError::PortNotConnected => {
                write!(f, "Port not connected")?;
            }
            AdsError::InvalidAmsLength => {
                write!(f, "Invalid AMS length")?;
            }
            AdsError::InvalidAmsNetID => {
                write!(f, "Invalid AMS NetID")?;
            }
            AdsError::LowInstallationLevel => {
                write!(f, "Low installation level")?;
            }
            AdsError::NoDebuggingAvailable => {
                write!(f, "No debugging available")?;
            }
            AdsError::PortDisabledSystemServiceNotStarted => {
                write!(f, "Port disabled - system service not started")?;
            }
            AdsError::PortAlreadyConnected => {
                write!(f, "Port already connected")?;
            }
            AdsError::AmsSyncWin32Error => {
                write!(f, "AMS Sync Win32 error")?;
            }
            AdsError::AmsSyncTimeout => {
                write!(f, "AMS Sync timeout")?;
            }
            AdsError::AmsSyncError => {
                write!(f, "AMS Sync error")?;
            }
            AdsError::AmsSyncNoIndexMap => {
                write!(f, "AMS Sync no index map")?;
            }
            AdsError::InvalidAmsPort => {
                write!(f, "Invalid AMS port")?;
            }
            AdsError::NoMemory => {
                write!(f, "No memory")?;
            }
            AdsError::TcpSendError => {
                write!(f, "TCP send error")?;
            }
            AdsError::HostUnreachable => {
                write!(f, "Host unreachable")?;
            }
            AdsError::InvalidAmsFragment => {
                write!(f, "Invalid AMS fragment")?;
            }
            AdsError::TlsSendErrorSecureAdsConnectionFailed => {
                write!(f, "TLS send error - secure ADS connection failed")?;
            }
            AdsError::AccessDeniedSecureAdsAccessDenied => {
                write!(f, "Access denied - secure ADS access denied")?;
            }
            AdsError::RouterNoLockedMemory => {
                write!(f, "Router: no locked memory")?;
            }
            AdsError::RouterMemorySizeCouldNotBeChanged => {
                write!(f, "Router: memory size could not be changed")?;
            }
            AdsError::RouterMailboxFull => {
                write!(f, "Router: mailbox full")?;
            }
            AdsError::RouterDebugMailboxFull => {
                write!(f, "Router: debug mailbox full")?;
            }
            AdsError::RouterPortTypeIsUnknown => {
                write!(f, "Router: port type is unknown")?;
            }
            AdsError::RouterIsNotInitialized => {
                write!(f, "Router is not initialized")?;
            }
            AdsError::RouterDesiredPortNumberIsAlreadyAssigned => {
                write!(f, "Router: desired port number is already assigned")?;
            }
            AdsError::RouterPortNotRegistered => {
                write!(f, "Router: port not registered")?;
            }
            AdsError::RouterMaximumNumberOfPortsReached => {
                write!(f, "Router: maximum number of ports reached")?;
            }
            AdsError::RouterPortIsInvalid => {
                write!(f, "Router: port is invalid")?;
            }
            AdsError::RouterIsNotActive => {
                write!(f, "Router is not active")?;
            }
            AdsError::RouterMailboxFullForFragmentedMessages => {
                write!(f, "Router: mailbox full for fragmented messages")?;
            }
            AdsError::RouterFragmentTimeoutOccurred => {
                write!(f, "Router: fragment timeout occurred")?;
            }
            AdsError::RouterPortRemoved => {
                write!(f, "Router: port removed")?;
            }
            AdsError::GeneralDeviceError => {
                write!(f, "General device error")?;
            }
            AdsError::ServiceIsNotSupportedByServer => {
                write!(f, "Service is not supported by server")?;
            }
            AdsError::InvalidIndexGroup => {
                write!(f, "Invalid index group")?;
            }
            AdsError::InvalidIndexOffset => {
                write!(f, "Invalid index offset")?;
            }
            AdsError::ReadingWritingNotPermitted => {
                write!(f, "Reading/writing not permitted")?;
            }
            AdsError::ParameterSizeNotCorrect => {
                write!(f, "Parameter size not correct")?;
            }
            AdsError::InvalidParameterValueS => {
                write!(f, "Invalid parameter value(s)")?;
            }
            AdsError::DeviceIsNotInAReadyState => {
                write!(f, "Device is not in a ready state")?;
            }
            AdsError::DeviceIsBusy => {
                write!(f, "Device is busy")?;
            }
            AdsError::InvalidOsContextUseMultiTaskDataAccess => {
                write!(f, "Invalid OS context -> use multi-task data access")?;
            }
            AdsError::OutOfMemory => {
                write!(f, "Out of memory")?;
            }
            AdsError::InvalidParameterValueS1 => {
                write!(f, "Invalid parameter value(s)")?;
            }
            AdsError::NotFoundFiles => {
                write!(f, "Not found (files,...)")?;
            }
            AdsError::SyntaxErrorInCommandOrFile => {
                write!(f, "Syntax error in command or file")?;
            }
            AdsError::ObjectsDoNotMatch => {
                write!(f, "Objects do not match")?;
            }
            AdsError::ObjectAlreadyExists => {
                write!(f, "Object already exists")?;
            }
            AdsError::SymbolNotFound => {
                write!(f, "Symbol not found")?;
            }
            AdsError::SymbolVersionInvalidCreateANewHandle => {
                write!(f, "Symbol version invalid -> create a new handle")?;
            }
            AdsError::ServerIsInAnInvalidState => {
                write!(f, "Server is in an invalid state")?;
            }
            AdsError::AdsTransModeNotSupported => {
                write!(f, "AdsTransMode not supported")?;
            }
            AdsError::NotificationHandleIsInvalid => {
                write!(f, "Notification handle is invalid")?;
            }
            AdsError::NotificationClientNotRegistered => {
                write!(f, "Notification client not registered")?;
            }
            AdsError::NoMoreNotificationHandles => {
                write!(f, "No more notification handles")?;
            }
            AdsError::NotificationSizeTooLarge => {
                write!(f, "Notification size too large")?;
            }
            AdsError::DeviceNotInitialized => {
                write!(f, "Device not initialized")?;
            }
            AdsError::DeviceHasATimeout => {
                write!(f, "Device has a timeout")?;
            }
            AdsError::QueryInterfaceFailed => {
                write!(f, "Query interface failed")?;
            }
            AdsError::WrongInterfaceRequired => {
                write!(f, "Wrong interface required")?;
            }
            AdsError::ClassIdIsInvalid => {
                write!(f, "Class ID is invalid")?;
            }
            AdsError::ObjectIdIsInvalid => {
                write!(f, "Object ID is invalid")?;
            }
            AdsError::RequestIsPending => {
                write!(f, "Request is pending")?;
            }
            AdsError::RequestIsAborted => {
                write!(f, "Request is aborted")?;
            }
            AdsError::SignalWarning => {
                write!(f, "Signal warning")?;
            }
            AdsError::InvalidArrayIndex => {
                write!(f, "Invalid array index")?;
            }
            AdsError::SymbolNotActiveReleaseHandleAndTryAgain => {
                write!(f, "Symbol not active -> release handle and try again")?;
            }
            AdsError::AccessDenied => {
                write!(f, "Access denied")?;
            }
            AdsError::NoLicenseFoundActivateLicense => {
                write!(f, "No license found -> activate license")?;
            }
            AdsError::LicenseExpired => {
                write!(f, "License expired")?;
            }
            AdsError::LicenseExceeded => {
                write!(f, "License exceeded")?;
            }
            AdsError::LicenseInvalid => {
                write!(f, "License invalid")?;
            }
            AdsError::InvalidSystemIdInLicense => {
                write!(f, "Invalid system ID in license")?;
            }
            AdsError::LicenseNotTimeLimited => {
                write!(f, "License not time limited")?;
            }
            AdsError::LicenseIssueTimeInTheFuture => {
                write!(f, "License issue time in the future")?;
            }
            AdsError::LicenseTimePeriodTooLong => {
                write!(f, "License time period too long")?;
            }
            AdsError::ExceptionInDeviceSpecificCodeCheckEachDevice => {
                write!(f, "Exception in device specific code -> check each device")?;
            }
            AdsError::LicenseFileReadTwice => {
                write!(f, "License file read twice")?;
            }
            AdsError::InvalidSignature => {
                write!(f, "Invalid signature")?;
            }
            AdsError::InvalidPublicKeyCertificate => {
                write!(f, "Invalid public key certificate")?;
            }
            AdsError::PublicKeyNotKnownFromOem => {
                write!(f, "Public key not known from OEM")?;
            }
            AdsError::LicenseNotValidForThisSystemId => {
                write!(f, "License not valid for this system ID")?;
            }
            AdsError::DemoLicenseProhibited => {
                write!(f, "Demo license prohibited")?;
            }
            AdsError::InvalidFunctionId => {
                write!(f, "Invalid function ID")?;
            }
            AdsError::OutsideTheValidRange => {
                write!(f, "Outside the valid range")?;
            }
            AdsError::InvalidAlignment => {
                write!(f, "Invalid alignment")?;
            }
            AdsError::InvalidPlatformLevel => {
                write!(f, "Invalid platform level")?;
            }
            AdsError::ContextForwardToPassiveLevel => {
                write!(f, "Context - forward to passive level")?;
            }
            AdsError::ContentForwardToDispatchLevel => {
                write!(f, "Content - forward to dispatch level")?;
            }
            AdsError::ContextForwardToRealTime => {
                write!(f, "Context - forward to real-time")?;
            }
            AdsError::GeneralClientError => {
                write!(f, "General client error")?;
            }
            AdsError::InvalidParameterAtService => {
                write!(f, "Invalid parameter at service")?;
            }
            AdsError::PollingListIsEmpty => {
                write!(f, "Polling list is empty")?;
            }
            AdsError::VarConnectionAlreadyInUse => {
                write!(f, "Var connection already in use")?;
            }
            AdsError::InvokeIdInUse => {
                write!(f, "Invoke ID in use")?;
            }
            AdsError::TimeoutElapsedCheckRouteSetting => {
                write!(f, "Timeout elapsed -> check route setting")?;
            }
            AdsError::ErrorInWin32Subsystem => {
                write!(f, "Error in Win32 subsystem")?;
            }
            AdsError::InvalidClientTimeoutValue => {
                write!(f, "Invalid client timeout value")?;
            }
            AdsError::AdsPortNotOpened => {
                write!(f, "ADS port not opened")?;
            }
            AdsError::NoAmsAddress => {
                write!(f, "No AMS address")?;
            }
            AdsError::InternalErrorInAdsSync => {
                write!(f, "Internal error in ADS sync")?;
            }
            AdsError::HashTableOverflow => {
                write!(f, "Hash table overflow")?;
            }
            AdsError::KeyNotFoundInHash => {
                write!(f, "Key not found in hash")?;
            }
            AdsError::NoMoreSymbolsInCache => {
                write!(f, "No more symbols in cache")?;
            }
            AdsError::InvalidResponseReceived => {
                write!(f, "Invalid response received")?;
            }
            AdsError::SyncPortIsLocked => {
                write!(f, "Sync port is locked")?;
            }
            AdsError::InternalErrorInRealTimeSystem => {
                write!(f, "Internal error in real-time system")?;
            }
            AdsError::TimerValueNotValid => {
                write!(f, "Timer value not valid")?;
            }
            AdsError::TaskPointerHasInvalidValue0 => {
                write!(f, "Task pointer has invalid value 0")?;
            }
            AdsError::StackPointerHasInvalidValue0 => {
                write!(f, "Stack pointer has invalid value 0")?;
            }
            AdsError::RequestedTaskPriorityAlreadyAssigned => {
                write!(f, "Requested task priority already assigned")?;
            }
            AdsError::NoFreeTaskControlBlock => {
                write!(f, "No free Task Control Block")?;
            }
            AdsError::NoFreeSemaphores => {
                write!(f, "No free semaphores")?;
            }
            AdsError::NoFreeSpaceInTheQueue => {
                write!(f, "No free space in the queue")?;
            }
            AdsError::ExternalSyncInterruptAlreadyApplied => {
                write!(f, "External sync interrupt already applied")?;
            }
            AdsError::NoExternalSyncInterruptApplied => {
                write!(f, "No external sync interrupt applied")?;
            }
            AdsError::ExternalSyncInterruptApplicationFailed => {
                write!(f, "External sync interrupt application failed")?;
            }
            AdsError::CallOfServiceFunctionInWrongContext => {
                write!(f, "Call of service function in wrong context")?;
            }
            AdsError::IntelVtXNotSupported => {
                write!(f, "Intel VT-x not supported")?;
            }
            AdsError::IntelVtXNotEnabledInBios => {
                write!(f, "Intel VT-x not enabled in BIOS")?;
            }
            AdsError::MissingFunctionInIntelVtX => {
                write!(f, "Missing function in Intel VT-x")?;
            }
            AdsError::ActivationOfIntelVtXFailed => {
                write!(f, "Activation of Intel VT-x failed")?;
            }
            AdsError::Unknown => {
                write!(f, "unknown")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum Command {
    DevInfo = 1,
    Read = 2,
    Write = 3,
    ReadState = 4,
    WriteControl = 5,
    AddNotification = 6,
    DeleteNotification = 7,
    Notification = 8,
    ReadWrite = 9,
    Unknown = 0xFFFF,
}

impl From<u16> for Command {
    #[allow(clippy::too_many_lines)]
    fn from(v: u16) -> Self {
        match v {
            x if x == Command::DevInfo as u16 => Command::DevInfo,
            x if x == Command::Read as u16 => Command::Read,
            x if x == Command::Write as u16 => Command::Write,
            x if x == Command::ReadState as u16 => Command::ReadState,
            x if x == Command::WriteControl as u16 => Command::WriteControl,
            x if x == Command::AddNotification as u16 => Command::AddNotification,
            x if x == Command::DeleteNotification as u16 => Command::DeleteNotification,
            x if x == Command::Notification as u16 => Command::Notification,
            x if x == Command::ReadWrite as u16 => Command::ReadWrite,
            _ => Command::Unknown,
        }
    }
}

impl std::fmt::Display for Command {
    #[allow(clippy::too_many_lines, clippy::match_same_arms)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::DevInfo => {
                write!(f, "dev info")?;
            }
            Command::Read => {
                write!(f, "read")?;
            }
            Command::Write => {
                write!(f, "write")?;
            }
            Command::ReadState => {
                write!(f, "read state")?;
            }
            Command::WriteControl => {
                write!(f, "write control")?;
            }
            Command::AddNotification => {
                write!(f, "add notification")?;
            }
            Command::DeleteNotification => {
                write!(f, "delete notification")?;
            }
            Command::Notification => {
                write!(f, "notification")?;
            }
            Command::ReadWrite => {
                write!(f, "read write")?;
            }
            Command::Unknown => {
                write!(f, "unknown")?;
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub const DATA_TYPES: &[DataType] = &[
    DataType::Void,
    DataType::Int8,
    DataType::Uint8,
    DataType::Int16,
    DataType::Uint16,
    DataType::Int32,
    DataType::Uint32,
    DataType::Int64,
    DataType::Uint64,
    DataType::Real32,
    DataType::Real64,
    DataType::Bigtype,
    DataType::String,
    DataType::Wstring,
    DataType::Real80,
    DataType::Bit,
    DataType::Maxtypes,
];

#[allow(dead_code)]
pub static DATA_TYPES_NAMES_LEN: Lazy<usize> =
    Lazy::new(|| DATA_TYPES.iter().map(|v| v.as_str().as_bytes().len()).sum());

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite, Serialize, Deserialize)]
#[brw(repr = u32)]
#[repr(u32)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    #[serde(alias = "NULL")]
    Void = 0,
    #[serde(alias = "SINT")]
    Int8 = 16,
    #[serde(alias = "USINT")]
    Uint8 = 17,
    #[serde(alias = "INT")]
    Int16 = 2,
    #[serde(alias = "UINT")]
    Uint16 = 18,
    #[serde(alias = "DINT")]
    Int32 = 3,
    #[serde(alias = "UDINT")]
    Uint32 = 19,
    #[serde(alias = "LINT")]
    Int64 = 20,
    #[serde(alias = "ULINT")]
    Uint64 = 21,
    #[serde(alias = "REAL")]
    Real32 = 4,
    #[serde(alias = "LREAL")]
    Real64 = 5,
    #[serde(alias = "BIG")]
    Bigtype = 65,
    #[serde(alias = "STRING")]
    String = 30,
    #[serde(alias = "WSTRING")]
    Wstring = 31,
    #[serde(alias = "REAL80")]
    Real80 = 32,
    #[serde(alias = "BIT")]
    Bit = 33,
    #[serde(alias = "MAX")]
    Maxtypes = 34,
    #[serde(alias = "UNKNOWN")]
    Unknown = 0xFFFF,
}

// SymInfo len = 42 + 3 + sym_name.len() * 2

#[binrw]
#[brw(little)]
struct SymInfoEx {
    length: u32,
    version: u32,
    subitem_index: u16,
    plc_interface_id: u16,
    reserved: u32,
    size: u32,
    offset: u32,
    base_type: u32,
    flags: u32,
    len_name: u16,
    len_type: u16,
    len_comment: u16,
    array_dim: u16,
    sub_items: u16,
}

impl DataType {
    pub fn packed_info_ex(self) -> Result<Vec<u8>, AdsError> {
        let sym_name = self.as_str().as_bytes();
        let length = 45 + sym_name.len() * 2;
        let mut buf = Cursor::new(Vec::with_capacity(length));
        let info = SymInfoEx {
            length: u32::try_from(length)?,
            version: 1,
            subitem_index: 0,
            plc_interface_id: 0,
            reserved: 0,
            size: u32::try_from(self.size())?,
            offset: 0,
            base_type: self as u32,
            flags: 0,
            len_name: u16::try_from(sym_name.len())?,
            len_type: u16::try_from(sym_name.len())?,
            len_comment: 0,
            array_dim: 0,
            sub_items: 0,
        };
        info.write(&mut buf)?;
        buf.write_all(sym_name)?;
        buf.write_all(&[0x20])?;
        buf.write_all(sym_name)?;
        buf.write_all(&[0x20])?;
        buf.write_all(&[0x20])?;
        Ok(buf.into_inner())
    }
    pub fn size(self) -> usize {
        match self {
            DataType::Int16 | DataType::Uint16 => 2,
            DataType::Int32 | DataType::Uint32 | DataType::Real32 => 4,
            DataType::Int64 | DataType::Uint64 | DataType::Real64 => 8,
            DataType::Real80 => 10,
            _ => 1,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            DataType::Void => "VOID",
            DataType::Int8 => "INT8",
            DataType::Uint8 => "UINT8",
            DataType::Int16 => "INT16",
            DataType::Uint16 => "UINT16",
            DataType::Int32 => "INT32",
            DataType::Uint32 => "UINT32",
            DataType::Int64 => "INT64",
            DataType::Uint64 => "UINT64",
            DataType::Real32 => "REAL32",
            DataType::Real64 => "REAL64",
            DataType::Bigtype => "BIGTYPE",
            DataType::String => "STRING",
            DataType::Wstring => "WSTRING",
            DataType::Real80 => "REAL80",
            DataType::Bit => "BIT",
            DataType::Maxtypes => "MAXTYPES",
            DataType::Unknown => "UNKNOWN",
        }
    }
}

impl From<u32> for DataType {
    #[allow(clippy::too_many_lines)]
    fn from(v: u32) -> Self {
        match v {
            x if x == DataType::Void as u32 => DataType::Void,
            x if x == DataType::Int8 as u32 => DataType::Int8,
            x if x == DataType::Uint8 as u32 => DataType::Uint8,
            x if x == DataType::Int16 as u32 => DataType::Int16,
            x if x == DataType::Uint16 as u32 => DataType::Uint16,
            x if x == DataType::Int32 as u32 => DataType::Int32,
            x if x == DataType::Uint32 as u32 => DataType::Uint32,
            x if x == DataType::Int64 as u32 => DataType::Int64,
            x if x == DataType::Uint64 as u32 => DataType::Uint64,
            x if x == DataType::Real32 as u32 => DataType::Real32,
            x if x == DataType::Real64 as u32 => DataType::Real64,
            x if x == DataType::Bigtype as u32 => DataType::Bigtype,
            x if x == DataType::String as u32 => DataType::String,
            x if x == DataType::Wstring as u32 => DataType::Wstring,
            x if x == DataType::Real80 as u32 => DataType::Real80,
            x if x == DataType::Bit as u32 => DataType::Bit,
            x if x == DataType::Maxtypes as u32 => DataType::Maxtypes,
            _ => DataType::Unknown,
        }
    }
}

impl std::fmt::Display for DataType {
    #[allow(clippy::too_many_lines, clippy::match_same_arms)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum AdsIGrp {
    Symtab = 0xF000,
    Symname = 0xF001,
    Symval = 0xF002,
    SymHndbyname = 0xF003,
    SymValbyname = 0xF004,
    SymValbyhnd = 0xF005,
    SymReleasehnd = 0xF006,
    SymInfobyname = 0xF007,
    SymVersion = 0xF008,
    SymInfobynameex = 0xF009,
    SymDownload = 0xF00A,
    SymUpload = 0xF00B,
    SymUploadinfo = 0xF00C,
    SymDownload2 = 0xF00D,
    SymDtUpload = 0xF00E,
    SymUploadinfo2 = 0xF00F,
    Symnote = 0xF010,
    IoimageRwib = 0xF020,
    IoimageRwix = 0xF021,
    IoimageRwob = 0xF030,
    IoimageRwox = 0xF031,
    IoimageCleari = 0xF040,
    IoimageClearo = 0xF050,
    SumupRead = 0xF080,
    SumupWrite = 0xF081,
    SumupReadWrite = 0xF082,
    SumupReadEx = 0xF083,
    DeviceData = 0xF100,
    Unknown = 0xFFFF_FFFF,
}

impl From<u32> for AdsIGrp {
    #[allow(clippy::too_many_lines)]
    fn from(v: u32) -> Self {
        match v {
            x if x == AdsIGrp::Symtab as u32 => AdsIGrp::Symtab,
            x if x == AdsIGrp::Symname as u32 => AdsIGrp::Symname,
            x if x == AdsIGrp::Symval as u32 => AdsIGrp::Symval,
            x if x == AdsIGrp::SymHndbyname as u32 => AdsIGrp::SymHndbyname,
            x if x == AdsIGrp::SymValbyname as u32 => AdsIGrp::SymValbyname,
            x if x == AdsIGrp::SymValbyhnd as u32 => AdsIGrp::SymValbyhnd,
            x if x == AdsIGrp::SymReleasehnd as u32 => AdsIGrp::SymReleasehnd,
            x if x == AdsIGrp::SymInfobyname as u32 => AdsIGrp::SymInfobyname,
            x if x == AdsIGrp::SymVersion as u32 => AdsIGrp::SymVersion,
            x if x == AdsIGrp::SymInfobynameex as u32 => AdsIGrp::SymInfobynameex,
            x if x == AdsIGrp::SymDownload as u32 => AdsIGrp::SymDownload,
            x if x == AdsIGrp::SymUpload as u32 => AdsIGrp::SymUpload,
            x if x == AdsIGrp::SymUploadinfo as u32 => AdsIGrp::SymUploadinfo,
            x if x == AdsIGrp::SymDownload2 as u32 => AdsIGrp::SymDownload2,
            x if x == AdsIGrp::SymDtUpload as u32 => AdsIGrp::SymDtUpload,
            x if x == AdsIGrp::SymUploadinfo2 as u32 => AdsIGrp::SymUploadinfo2,
            x if x == AdsIGrp::Symnote as u32 => AdsIGrp::Symnote,
            x if x == AdsIGrp::IoimageRwib as u32 => AdsIGrp::IoimageRwib,
            x if x == AdsIGrp::IoimageRwix as u32 => AdsIGrp::IoimageRwix,
            x if x == AdsIGrp::IoimageRwob as u32 => AdsIGrp::IoimageRwob,
            x if x == AdsIGrp::IoimageRwox as u32 => AdsIGrp::IoimageRwox,
            x if x == AdsIGrp::IoimageCleari as u32 => AdsIGrp::IoimageCleari,
            x if x == AdsIGrp::IoimageClearo as u32 => AdsIGrp::IoimageClearo,
            x if x == AdsIGrp::SumupRead as u32 => AdsIGrp::SumupRead,
            x if x == AdsIGrp::SumupWrite as u32 => AdsIGrp::SumupWrite,
            x if x == AdsIGrp::SumupReadWrite as u32 => AdsIGrp::SumupReadWrite,
            x if x == AdsIGrp::SumupReadEx as u32 => AdsIGrp::SumupReadEx,
            x if x == AdsIGrp::DeviceData as u32 => AdsIGrp::DeviceData,
            _ => AdsIGrp::Unknown,
        }
    }
}

impl std::fmt::Display for AdsIGrp {
    #[allow(clippy::too_many_lines, clippy::match_same_arms)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdsIGrp::Symtab => {
                write!(f, "SYMTAB")?;
            }
            AdsIGrp::Symname => {
                write!(f, "SYMNAME")?;
            }
            AdsIGrp::Symval => {
                write!(f, "SYMVAL")?;
            }
            AdsIGrp::SymHndbyname => {
                write!(f, "SYM_HNDBYNAME")?;
            }
            AdsIGrp::SymValbyname => {
                write!(f, "SYM_VALBYNAME")?;
            }
            AdsIGrp::SymValbyhnd => {
                write!(f, "SYM_VALBYHND")?;
            }
            AdsIGrp::SymReleasehnd => {
                write!(f, "SYM_RELEASEHND")?;
            }
            AdsIGrp::SymInfobyname => {
                write!(f, "SYM_INFOBYNAME")?;
            }
            AdsIGrp::SymVersion => {
                write!(f, "SYM_VERSION")?;
            }
            AdsIGrp::SymInfobynameex => {
                write!(f, "SYM_INFOBYNAMEEX")?;
            }
            AdsIGrp::SymDownload => {
                write!(f, "SYM_DOWNLOAD")?;
            }
            AdsIGrp::SymUpload => {
                write!(f, "SYM_UPLOAD")?;
            }
            AdsIGrp::SymUploadinfo => {
                write!(f, "SYM_UPLOADINFO")?;
            }
            AdsIGrp::SymDownload2 => {
                write!(f, "SYM_DOWNLOAD2")?;
            }
            AdsIGrp::SymDtUpload => {
                write!(f, "SYM_DT_UPLOAD")?;
            }
            AdsIGrp::SymUploadinfo2 => {
                write!(f, "SYM_UPLOADINFO2")?;
            }
            AdsIGrp::Symnote => {
                write!(f, "SYMNOTE")?;
            }
            AdsIGrp::IoimageRwib => {
                write!(f, "IOIMAGE_RWIB")?;
            }
            AdsIGrp::IoimageRwix => {
                write!(f, "IOIMAGE_RWIX")?;
            }
            AdsIGrp::IoimageRwob => {
                write!(f, "IOIMAGE_RWOB")?;
            }
            AdsIGrp::IoimageRwox => {
                write!(f, "IOIMAGE_RWOX")?;
            }
            AdsIGrp::IoimageCleari => {
                write!(f, "IOIMAGE_CLEARI")?;
            }
            AdsIGrp::IoimageClearo => {
                write!(f, "IOIMAGE_CLEARO")?;
            }
            AdsIGrp::SumupRead => {
                write!(f, "SUMUP_READ")?;
            }
            AdsIGrp::SumupWrite => {
                write!(f, "SUMUP_WRITE")?;
            }
            AdsIGrp::SumupReadWrite => {
                write!(f, "SUMUP_READ_WRITE")?;
            }
            AdsIGrp::SumupReadEx => {
                write!(f, "SUMUP_READ_EX")?;
            }
            AdsIGrp::DeviceData => {
                write!(f, "DEVICE_DATA")?;
            }
            AdsIGrp::Unknown => {
                write!(f, "unknown")?;
            }
        }
        Ok(())
    }
}
