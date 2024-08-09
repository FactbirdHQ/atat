/// Enumeration of Mobile Equipment errors, as defined in 3GPP TS 27.007
/// v17.1.0, section 9.2 (Mobile termination error result code +CME ERROR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CmeError {
    /// nick=PhoneFailure
    PhoneFailure = 0,
    /// nick=NoConnection
    NoConnection = 1,
    /// nick=LinkReserved
    LinkReserved = 2,
    /// nick=NotAllowed
    NotAllowed = 3,
    /// nick=NotSupported
    NotSupported = 4,
    /// nick=PhSimPin
    PhSimPin = 5,
    /// nick=PhFsimPin
    PhFsimPin = 6,
    /// nick=PhFsimPuk
    PhFsimPuk = 7,
    /// nick=SimNotInserted
    SimNotInserted = 10,
    /// nick=SimPin
    SimPin = 11,
    /// nick=SimPuk
    SimPuk = 12,
    /// nick=SimFailure
    SimFailure = 13,
    /// nick=SimBusy
    SimBusy = 14,
    /// nick=SimWrong
    SimWrong = 15,
    /// nick=IncorrectPassword
    IncorrectPassword = 16,
    /// nick=SimPin2
    SimPin2 = 17,
    /// nick=SimPuk2
    SimPuk2 = 18,
    /// nick=MemoryFull
    MemoryFull = 20,
    /// nick=InvalidIndex
    InvalidIndex = 21,
    /// nick=NotFound
    NotFound = 22,
    /// nick=MemoryFailure
    MemoryFailure = 23,
    /// nick=TextTooLong
    TextTooLong = 24,
    /// nick=InvalidChars
    InvalidChars = 25,
    /// nick=DialStringTooLong
    DialStringTooLong = 26,
    /// nick=DialStringInvalid
    DialStringInvalid = 27,
    /// nick=NoNetwork
    NoNetwork = 30,
    /// nick=NetworkTimeout
    NetworkTimeout = 31,
    /// nick=NetworkNotAllowed
    NetworkNotAllowed = 32,
    /// nick=NetworkPin
    NetworkPin = 40,
    /// nick=NetworkPuk
    NetworkPuk = 41,
    /// nick=NetworkSubsetPin
    NetworkSubsetPin = 42,
    /// nick=NetworkSubsetPuk
    NetworkSubsetPuk = 43,
    /// nick=ServicePin
    ServicePin = 44,
    /// nick=ServicePuk
    ServicePuk = 45,
    /// nick=CorpPin
    CorpPin = 46,
    /// nick=CorpPuk
    CorpPuk = 47,
    /// nick=HiddenKeyRequired
    HiddenKeyRequired = 48,
    /// nick=EapMethodNotSupported
    EapMethodNotSupported = 49,
    /// nick=IncorrectParameters
    IncorrectParameters = 50,
    /// nick=CommandDisabled
    CommandDisabled = 51,
    /// nick=CommandAborted
    CommandAborted = 52,
    /// nick=NotAttachedRestricted
    NotAttachedRestricted = 53,
    /// nick=NotAllowedEmergencyOnly
    NotAllowedEmergencyOnly = 54,
    /// nick=NotAllowedRestricted
    NotAllowedRestricted = 55,
    /// nick=FixedDialNumberOnly
    FixedDialNumberOnly = 56,
    /// nick=TemporarilyOutOfService
    TemporarilyOutOfService = 57,
    /// nick=LanguageOrAlphabetNotSupported
    LanguageOrAlphabetNotSupported = 58,
    /// nick=UnexpectedDataValue
    UnexpectedDataValue = 59,
    /// nick=SystemFailure
    SystemFailure = 60,
    /// nick=DataMissing
    DataMissing = 61,
    /// nick=CallBarred
    CallBarred = 62,
    /// nick=MessageWaitingIndicationSubscriptionFailure
    MessageWaitingIndicationSubscriptionFailure = 63,
    /// nick=Unknown
    Unknown = 100,
    /// nick=ImsiUnknownInHss
    ImsiUnknownInHss = 102,
    /// nick=IllegalUe
    IllegalUe = 103,
    /// nick=ImsiUnknownInVlr
    ImsiUnknownInVlr = 104,
    /// nick=ImeiNotAccepted
    ImeiNotAccepted = 105,
    /// nick=IllegalMe
    IllegalMe = 106,
    /// nick=PsServicesNotAllowed
    PsServicesNotAllowed = 107,
    /// nick=PsAndNonPsServicesNotAllowed
    PsAndNonPsServicesNotAllowed = 108,
    /// nick=UeIdentityNotDerivedFromNetwork
    UeIdentityNotDerivedFromNetwork = 109,
    /// nick=ImplicitlyDetached
    ImplicitlyDetached = 110,
    /// nick=PlmnNotAllowed
    PlmnNotAllowed = 111,
    /// nick=AreaNotAllowed
    AreaNotAllowed = 112,
    /// nick=RoamingNotAllowedInArea
    RoamingNotAllowedInArea = 113,
    /// nick=PsServicesNotAllowedInPlmn
    PsServicesNotAllowedInPlmn = 114,
    /// nick=NoCellsInArea
    NoCellsInArea = 115,
    /// nick=MscTemporarilyNotReachable
    MscTemporarilyNotReachable = 116,
    /// nick=NetworkFailureAttach
    NetworkFailureAttach = 117,
    /// nick=CsDomainUnavailable
    CsDomainUnavailable = 118,
    /// nick=EsmFailure
    EsmFailure = 119,
    /// nick=Congestion
    Congestion = 122,
    /// nick=MbmsBearerCapabilitiesInsufficientForService
    MbmsBearerCapabilitiesInsufficientForService = 124,
    /// nick=NotAuthorizedForCsg
    NotAuthorizedForCsg = 125,
    /// nick=InsufficientResources
    InsufficientResources = 126,
    /// nick=MissingOrUnknownApn
    MissingOrUnknownApn = 127,
    /// nick=UnknownPdpAddressOrType
    UnknownPdpAddressOrType = 128,
    /// nick=UserAuthenticationFailed
    UserAuthenticationFailed = 129,
    /// nick=ActivationRejectedByGgsnOrGw
    ActivationRejectedByGgsnOrGw = 130,
    /// nick=ActivationRejectedUnspecified
    ActivationRejectedUnspecified = 131,
    /// nick=ServiceOptionNotSupported
    ServiceOptionNotSupported = 132,
    /// nick=ServiceOptionNotSubscribed
    ServiceOptionNotSubscribed = 133,
    /// nick=ServiceOptionOutOfOrder
    ServiceOptionOutOfOrder = 134,
    /// nick=NsapiOrPtiAlreadyInUse
    NsapiOrPtiAlreadyInUse = 135,
    /// nick=RegularDeactivation
    RegularDeactivation = 136,
    /// nick=QosNotAccepted
    QosNotAccepted = 137,
    /// nick=CallCannotBeIdentified
    CallCannotBeIdentified = 138,
    /// nick=CsServiceTemporarilyUnavailable
    CsServiceTemporarilyUnavailable = 139,
    /// nick=FeatureNotSupported
    FeatureNotSupported = 140,
    /// nick=SemanticErrorInTftOperation
    SemanticErrorInTftOperation = 141,
    /// nick=SyntacticalErrorInTftOperation
    SyntacticalErrorInTftOperation = 142,
    /// nick=UnknownPdpContext
    UnknownPdpContext = 143,
    /// nick=SemanticErrorsInPacketFilter
    SemanticErrorsInPacketFilter = 144,
    /// nick=SyntacticalErrorsInPacketFilter
    SyntacticalErrorInPacketFilter = 145,
    /// nick=PdpContextWithoutTftAlreadyActivated
    PdpContextWithoutTftAlreadyActivated = 146,
    /// nick=MulticastGroupMembershipTimeout
    MulticastGroupMembershipTimeout = 147,
    /// nick=GprsUnknown
    GprsUnknown = 148,
    /// nick=PdpAuthFailure
    PdpAuthFailure = 149,
    /// nick=InvalidMobileClass
    InvalidMobileClass = 150,
    /// nick=LastPdnDisconnectionNotAllowedLegacy
    LastPdnDisconnectionNotAllowedLegacy = 151,
    /// nick=LastPdnDisconnectionNotAllowed
    LastPdnDisconnectionNotAllowed = 171,
    /// nick=SemanticallyIncorrectMessage
    SemanticallyIncorrectMessage = 172,
    /// nick=InvalidMandatoryInformation
    InvalidMandatoryInformation = 173,
    /// nick=MessageTypeNotImplemented
    MessageTypeNotImplemented = 174,
    /// nick=ConditionalIeError
    ConditionalIeError = 175,
    /// nick=UnspecifiedProtocolError
    UnspecifiedProtocolError = 176,
    /// nick=OperatorDeterminedBarring
    OperatorDeterminedBarring = 177,
    /// nick=MaximumNumberOfBearersReached
    MaximumNumberOfBearersReached = 178,
    /// nick=RequestedApnNotSupported
    RequestedApnNotSupported = 179,
    /// nick=RequestRejectedBcmViolation
    RequestRejectedBcmViolation = 180,
    /// nick=UnsupportedQciOr5qiValue
    UnsupportedQciOr5QiValue = 181,
    /// nick=UserDataViaControlPlaneCongested
    UserDataViaControlPlaneCongested = 182,
    /// nick=SmsProvidedViaGprsInRoutingArea
    SmsProvidedViaGprsInRoutingArea = 183,
    /// nick=InvalidPtiValue
    InvalidPtiValue = 184,
    /// nick=NoBearerActivated
    NoBearerActivated = 185,
    /// nick=MessageNotCompatibleWithProtocolState
    MessageNotCompatibleWithProtocolState = 186,
    /// nick=RecoveryOnTimerExpiry
    RecoveryOnTimerExpiry = 187,
    /// nick=InvalidTransactionIdValue
    InvalidTransactionIdValue = 188,
    /// nick=ServiceOptionNotAuthorizedInPlmn
    ServiceOptionNotAuthorizedInPlmn = 189,
    /// nick=NetworkFailureActivation
    NetworkFailureActivation = 190,
    /// nick=ReactivationRequested
    ReactivationRequested = 191,
    /// nick=Ipv4OnlyAllowed
    Ipv4OnlyAllowed = 192,
    /// nick=Ipv6OnlyAllowed
    Ipv6OnlyAllowed = 193,
    /// nick=SingleAddressBearersOnlyAllowed
    SingleAddressBearersOnlyAllowed = 194,
    /// nick=CollisionWithNetworkInitiatedRequest
    CollisionWithNetworkInitiatedRequest = 195,
    /// nick=Ipv4v6OnlyAllowed
    Ipv4V6OnlyAllowed = 196,
    /// nick=NonIpOnlyAllowed
    NonIpOnlyAllowed = 197,
    /// nick=BearerHandlingUnsupported
    BearerHandlingUnsupported = 198,
    /// nick=ApnRestrictionIncompatible
    ApnRestrictionIncompatible = 199,
    /// nick=MultipleAccessToPdnConnectionNotAllowed
    MultipleAccessToPdnConnectionNotAllowed = 200,
    /// nick=EsmInformationNotReceived
    EsmInformationNotReceived = 201,
    /// nick=PdnConnectionNonexistent
    PdnConnectionNonexistent = 202,
    /// nick=MultiplePdnConnectionSameApnNotAllowed
    MultiplePdnConnectionSameApnNotAllowed = 203,
    /// nick=SevereNetworkFailure
    SevereNetworkFailure = 204,
    /// nick=InsufficientResourcesForSliceAndDnn
    InsufficientResourcesForSliceAndDnn = 205,
    /// nick=UnsupportedSscMode
    UnsupportedSscMode = 206,
    /// nick=InsufficientResourcesForSlice
    InsufficientResourcesForSlice = 207,
    /// nick=MessageTypeNotCompatibleWithProtocolState
    MessageTypeNotCompatibleWithProtocolState = 208,
    /// nick=IeNotImplemented
    IeNotImplemented = 209,
    /// nick=N1ModeNotAllowed
    N1ModeNotAllowed = 210,
    /// nick=RestrictedServiceArea
    RestrictedServiceArea = 211,
    /// nick=LadnUnavailable
    LadnUnavailable = 212,
    /// nick=MissingOrUnknownDnnInSlice
    MissingOrUnknownDnnInSlice = 213,
    /// nick=NkgsiAlreadyInUse
    NgksiAlreadyInUse = 214,
    /// nick=PayloadNotForwarded
    PayloadNotForwarded = 215,
    /// nick=Non3gppAccessTo5gcnNotAllowed
    Non3GppAccessTo5GcnNotAllowed = 216,
    /// nick=ServingNetworkNotAuthorized
    ServingNetworkNotAuthorized = 217,
    /// nick=DnnNotSupportedInSlice
    DnnNotSupportedInSlice = 218,
    /// nick=InsufficientUserPlaneResourcesForPduSession
    InsufficientUserPlaneResourcesForPduSessio = 219,
    /// nick=OutOfLadnServiceArea
    OutOfLadnServiceArea = 220,
    /// nick=PtiMismatch
    PtiMismatch = 221,
    /// nick=MaxDataRateForUserPlaneIntegrityTooLow
    MaxDataRateForUserPlaneIntegrityTooLow = 222,
    /// nick=SemanticErrorInQosOperation
    SemanticErrorInQosOperation = 223,
    /// nick=SyntacticalErrorInQosOperation
    SyntacticalErrorInQosOperation = 224,
    /// nick=InvalidMappedEpsBearerIdentity
    InvalidMappedEpsBearerIdentity = 225,
    /// nick=RedirectionTo5gcnRequired
    RedirectionTo5GcnRequired = 226,
    /// nick=RedirectionToEpcRequired
    RedirectionToEpcRequired = 227,
    /// nick=TemporarilyUnauthorizedForSnpn
    TemporarilyUnauthorizedForSnpn = 228,
    /// nick=PermanentlyUnauthorizedForSnpn
    PermanentlyUnauthorizedForSnpn = 229,
    /// nick=EthernetOnlyAllowed
    EthernetOnlyAllowed = 230,
    /// nick=UnauthorizedForCag
    UnauthorizedForCag = 231,
    /// nick=NoNetworkSlicesAvailable
    NoNetworkSlicesAvailable = 232,
    /// nick=WirelineAccessAreaNotAllowed
    WirelineAccessAreaNotAllowed = 233,
}

impl From<u16> for CmeError {
    fn from(v: u16) -> Self {
        match v {
            0 => Self::PhoneFailure,
            1 => Self::NoConnection,
            2 => Self::LinkReserved,
            3 => Self::NotAllowed,
            4 => Self::NotSupported,
            5 => Self::PhSimPin,
            6 => Self::PhFsimPin,
            7 => Self::PhFsimPuk,
            10 => Self::SimNotInserted,
            11 => Self::SimPin,
            12 => Self::SimPuk,
            13 => Self::SimFailure,
            14 => Self::SimBusy,
            15 => Self::SimWrong,
            16 => Self::IncorrectPassword,
            17 => Self::SimPin2,
            18 => Self::SimPuk2,
            20 => Self::MemoryFull,
            21 => Self::InvalidIndex,
            22 => Self::NotFound,
            23 => Self::MemoryFailure,
            24 => Self::TextTooLong,
            25 => Self::InvalidChars,
            26 => Self::DialStringTooLong,
            27 => Self::DialStringInvalid,
            30 => Self::NoNetwork,
            31 => Self::NetworkTimeout,
            32 => Self::NetworkNotAllowed,
            40 => Self::NetworkPin,
            41 => Self::NetworkPuk,
            42 => Self::NetworkSubsetPin,
            43 => Self::NetworkSubsetPuk,
            44 => Self::ServicePin,
            45 => Self::ServicePuk,
            46 => Self::CorpPin,
            47 => Self::CorpPuk,
            48 => Self::HiddenKeyRequired,
            49 => Self::EapMethodNotSupported,
            50 => Self::IncorrectParameters,
            51 => Self::CommandDisabled,
            52 => Self::CommandAborted,
            53 => Self::NotAttachedRestricted,
            54 => Self::NotAllowedEmergencyOnly,
            55 => Self::NotAllowedRestricted,
            56 => Self::FixedDialNumberOnly,
            57 => Self::TemporarilyOutOfService,
            58 => Self::LanguageOrAlphabetNotSupported,
            59 => Self::UnexpectedDataValue,
            60 => Self::SystemFailure,
            61 => Self::DataMissing,
            62 => Self::CallBarred,
            63 => Self::MessageWaitingIndicationSubscriptionFailure,
            // 100 => Self::Unknown,
            102 => Self::ImsiUnknownInHss,
            103 => Self::IllegalUe,
            104 => Self::ImsiUnknownInVlr,
            105 => Self::ImeiNotAccepted,
            106 => Self::IllegalMe,
            107 => Self::PsServicesNotAllowed,
            108 => Self::PsAndNonPsServicesNotAllowed,
            109 => Self::UeIdentityNotDerivedFromNetwork,
            110 => Self::ImplicitlyDetached,
            111 => Self::PlmnNotAllowed,
            112 => Self::AreaNotAllowed,
            113 => Self::RoamingNotAllowedInArea,
            114 => Self::PsServicesNotAllowedInPlmn,
            115 => Self::NoCellsInArea,
            116 => Self::MscTemporarilyNotReachable,
            117 => Self::NetworkFailureAttach,
            118 => Self::CsDomainUnavailable,
            119 => Self::EsmFailure,
            122 => Self::Congestion,
            124 => Self::MbmsBearerCapabilitiesInsufficientForService,
            125 => Self::NotAuthorizedForCsg,
            126 => Self::InsufficientResources,
            127 => Self::MissingOrUnknownApn,
            128 => Self::UnknownPdpAddressOrType,
            129 => Self::UserAuthenticationFailed,
            130 => Self::ActivationRejectedByGgsnOrGw,
            131 => Self::ActivationRejectedUnspecified,
            132 => Self::ServiceOptionNotSupported,
            133 => Self::ServiceOptionNotSubscribed,
            134 => Self::ServiceOptionOutOfOrder,
            135 => Self::NsapiOrPtiAlreadyInUse,
            136 => Self::RegularDeactivation,
            137 => Self::QosNotAccepted,
            138 => Self::CallCannotBeIdentified,
            139 => Self::CsServiceTemporarilyUnavailable,
            140 => Self::FeatureNotSupported,
            141 => Self::SemanticErrorInTftOperation,
            142 => Self::SyntacticalErrorInTftOperation,
            143 => Self::UnknownPdpContext,
            144 => Self::SemanticErrorsInPacketFilter,
            145 => Self::SyntacticalErrorInPacketFilter,
            146 => Self::PdpContextWithoutTftAlreadyActivated,
            147 => Self::MulticastGroupMembershipTimeout,
            148 => Self::GprsUnknown,
            149 => Self::PdpAuthFailure,
            150 => Self::InvalidMobileClass,
            151 => Self::LastPdnDisconnectionNotAllowedLegacy,
            171 => Self::LastPdnDisconnectionNotAllowed,
            172 => Self::SemanticallyIncorrectMessage,
            173 => Self::InvalidMandatoryInformation,
            174 => Self::MessageTypeNotImplemented,
            175 => Self::ConditionalIeError,
            176 => Self::UnspecifiedProtocolError,
            177 => Self::OperatorDeterminedBarring,
            178 => Self::MaximumNumberOfBearersReached,
            179 => Self::RequestedApnNotSupported,
            180 => Self::RequestRejectedBcmViolation,
            181 => Self::UnsupportedQciOr5QiValue,
            182 => Self::UserDataViaControlPlaneCongested,
            183 => Self::SmsProvidedViaGprsInRoutingArea,
            184 => Self::InvalidPtiValue,
            185 => Self::NoBearerActivated,
            186 => Self::MessageNotCompatibleWithProtocolState,
            187 => Self::RecoveryOnTimerExpiry,
            188 => Self::InvalidTransactionIdValue,
            189 => Self::ServiceOptionNotAuthorizedInPlmn,
            190 => Self::NetworkFailureActivation,
            191 => Self::ReactivationRequested,
            192 => Self::Ipv4OnlyAllowed,
            193 => Self::Ipv6OnlyAllowed,
            194 => Self::SingleAddressBearersOnlyAllowed,
            195 => Self::CollisionWithNetworkInitiatedRequest,
            196 => Self::Ipv4V6OnlyAllowed,
            197 => Self::NonIpOnlyAllowed,
            198 => Self::BearerHandlingUnsupported,
            199 => Self::ApnRestrictionIncompatible,
            200 => Self::MultipleAccessToPdnConnectionNotAllowed,
            201 => Self::EsmInformationNotReceived,
            202 => Self::PdnConnectionNonexistent,
            203 => Self::MultiplePdnConnectionSameApnNotAllowed,
            204 => Self::SevereNetworkFailure,
            205 => Self::InsufficientResourcesForSliceAndDnn,
            206 => Self::UnsupportedSscMode,
            207 => Self::InsufficientResourcesForSlice,
            208 => Self::MessageTypeNotCompatibleWithProtocolState,
            209 => Self::IeNotImplemented,
            210 => Self::N1ModeNotAllowed,
            211 => Self::RestrictedServiceArea,
            212 => Self::LadnUnavailable,
            213 => Self::MissingOrUnknownDnnInSlice,
            214 => Self::NgksiAlreadyInUse,
            215 => Self::PayloadNotForwarded,
            216 => Self::Non3GppAccessTo5GcnNotAllowed,
            217 => Self::ServingNetworkNotAuthorized,
            218 => Self::DnnNotSupportedInSlice,
            219 => Self::InsufficientUserPlaneResourcesForPduSessio,
            220 => Self::OutOfLadnServiceArea,
            221 => Self::PtiMismatch,
            222 => Self::MaxDataRateForUserPlaneIntegrityTooLow,
            223 => Self::SemanticErrorInQosOperation,
            224 => Self::SyntacticalErrorInQosOperation,
            225 => Self::InvalidMappedEpsBearerIdentity,
            226 => Self::RedirectionTo5GcnRequired,
            227 => Self::RedirectionToEpcRequired,
            228 => Self::TemporarilyUnauthorizedForSnpn,
            229 => Self::PermanentlyUnauthorizedForSnpn,
            230 => Self::EthernetOnlyAllowed,
            231 => Self::UnauthorizedForCag,
            232 => Self::NoNetworkSlicesAvailable,
            233 => Self::WirelineAccessAreaNotAllowed,
            _ => Self::Unknown,
        }
    }
}

#[cfg(feature = "string_errors")]
impl CmeError {
    pub const fn from_msg(s: &[u8]) -> Self {
        // FIXME:
        match s {
            b"Phone failure" => Self::PhoneFailure,
            b"No connection to phone" => Self::NoConnection,
            b"Phone-adaptor link reserved" => Self::LinkReserved,
            b"Operation not allowed" => Self::NotAllowed,
            b"Operation not supported" => Self::NotSupported,
            b"SIM not inserted" => Self::SimNotInserted,
            b"SIM PIN required" => Self::SimPin,
            b"SIM PUK required" => Self::SimPuk,
            b"SIM failure" => Self::SimFailure,
            b"SIM busy" => Self::SimBusy,
            b"SIM wrong" => Self::SimWrong,
            b"Incorrect password" => Self::IncorrectPassword,
            b"Not found" => Self::NotFound,
            b"No network service" => Self::NoNetwork,
            b"Network timeout" => Self::NetworkTimeout,
            b"Incorrect parameters" => Self::IncorrectParameters,
            _ => Self::Unknown,
        }
    }
}

impl core::fmt::Display for CmeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PhoneFailure => write!(f, "Phone failure"),
            Self::NoConnection => write!(f, "No connection to phone"),
            Self::LinkReserved => write!(f, "Phone-adaptor link reserved"),
            Self::NotAllowed => write!(f, "Operation not allowed"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::PhSimPin => write!(f, "PH-SIM PIN required"),
            Self::PhFsimPin => write!(f, "PH-FSIM PIN required"),
            Self::PhFsimPuk => write!(f, "PH-FSIM PUK required"),
            Self::SimNotInserted => write!(f, "SIM not inserted"),
            Self::SimPin => write!(f, "SIM PIN required"),
            Self::SimPuk => write!(f, "SIM PUK required"),
            Self::SimFailure => write!(f, "SIM failure"),
            Self::SimBusy => write!(f, "SIM busy"),
            Self::SimWrong => write!(f, "SIM wrong"),
            Self::IncorrectPassword => write!(f, "Incorrect password"),
            Self::SimPin2 => write!(f, "SIM PIN2 required"),
            Self::SimPuk2 => write!(f, "SIM PUK2 required"),
            Self::MemoryFull => write!(f, "Memory full"),
            Self::InvalidIndex => write!(f, "Invalid index"),
            Self::NotFound => write!(f, "Not found"),
            Self::MemoryFailure => write!(f, "Memory failure"),
            Self::TextTooLong => write!(f, "Text string too long"),
            Self::InvalidChars => write!(f, "Invalid characters in text string"),
            Self::DialStringTooLong => write!(f, "Dial string too long"),
            Self::DialStringInvalid => write!(f, "Invalid characters in dial string"),
            Self::NoNetwork => write!(f, "No network service"),
            Self::NetworkTimeout => write!(f, "Network timeout"),
            Self::NetworkNotAllowed => write!(f, "Network not allowed - emergency calls only"),
            Self::NetworkPin => write!(f, "Network personalization PIN required"),
            Self::NetworkPuk => write!(f, "Network personalization PUK required"),
            Self::NetworkSubsetPin => write!(f, "Network subset personalization PIN required"),
            Self::NetworkSubsetPuk => write!(f, "Network subset personalization PUK required"),
            Self::ServicePin => write!(f, "Service provider personalization PIN required"),
            Self::ServicePuk => write!(f, "Service provider personalization PUK required"),
            Self::CorpPin => write!(f, "Corporate personalization PIN required"),
            Self::CorpPuk => write!(f, "Corporate personalization PUK required"),
            Self::HiddenKeyRequired => write!(f, "Hidden key required"),
            Self::EapMethodNotSupported => write!(f, "EAP method not supported"),
            Self::IncorrectParameters => write!(f, "Incorrect parameters"),
            Self::CommandDisabled => write!(f, "Command disabled"),
            Self::CommandAborted => write!(f, "Command aborted"),
            Self::NotAttachedRestricted => write!(f, "Not attached] restricted"),
            Self::NotAllowedEmergencyOnly => write!(f, "Not allowed] emergency only"),
            Self::NotAllowedRestricted => write!(f, "Not allowed] restricted"),
            Self::FixedDialNumberOnly => write!(f, "Fixed dial number only"),
            Self::TemporarilyOutOfService => write!(f, "Temporarily out of service"),
            Self::LanguageOrAlphabetNotSupported => write!(f, "Language or alphabet not supported"),
            Self::UnexpectedDataValue => write!(f, "Unexpected data value"),
            Self::SystemFailure => write!(f, "System failure"),
            Self::DataMissing => write!(f, "Data missing"),
            Self::CallBarred => write!(f, "Call barred"),
            Self::MessageWaitingIndicationSubscriptionFailure => {
                write!(f, "Message waiting indication subscription failure")
            }
            Self::Unknown => write!(f, "Unknown error"),
            Self::ImsiUnknownInHss => write!(f, "IMSI unknown in HLR/HSS"),
            Self::IllegalUe => write!(f, "Illegal MS/UE"),
            Self::ImsiUnknownInVlr => write!(f, "IMSI unknown in VLR"),
            Self::ImeiNotAccepted => write!(f, "IMEI not accepted"),
            Self::IllegalMe => write!(f, "Illegal ME"),
            Self::PsServicesNotAllowed => write!(f, "PS services not allowed"),
            Self::PsAndNonPsServicesNotAllowed => write!(f, "PS and non-PS services not allowed"),
            Self::UeIdentityNotDerivedFromNetwork => {
                write!(f, "UE identity not derived from network")
            }
            Self::ImplicitlyDetached => write!(f, "Implicitly detached"),
            Self::PlmnNotAllowed => write!(f, "PLMN not allowed"),
            Self::AreaNotAllowed => write!(f, "Location/tracking area not allowed"),
            Self::RoamingNotAllowedInArea => {
                write!(f, "Roaming not allowed in this location/tracking area")
            }
            Self::PsServicesNotAllowedInPlmn => write!(f, "PS services not allowed in PLMN"),
            Self::NoCellsInArea => write!(f, "No cells in location/tracking area"),
            Self::MscTemporarilyNotReachable => write!(f, "MSC temporarily not reachable"),
            Self::NetworkFailureAttach => write!(f, "Network failure (attach)"),
            Self::CsDomainUnavailable => write!(f, "CS domain unavailable"),
            Self::EsmFailure => write!(f, "ESM failure"),
            Self::Congestion => write!(f, "Congestion"),
            Self::MbmsBearerCapabilitiesInsufficientForService => {
                write!(f, "MBMS bearer capabilities insufficient for service")
            }
            Self::NotAuthorizedForCsg => write!(f, "Not authorized for CSG"),
            Self::InsufficientResources => write!(f, "Insufficient resources"),
            Self::MissingOrUnknownApn => write!(f, "Missing or unknown APN"),
            Self::UnknownPdpAddressOrType => write!(f, "Unknown PDP address or type"),
            Self::UserAuthenticationFailed => write!(f, "User authentication failed"),
            Self::ActivationRejectedByGgsnOrGw => write!(f, "Activation rejected by GGSN or GW"),
            Self::ActivationRejectedUnspecified => write!(f, "Activation rejected (unspecified)"),
            Self::ServiceOptionNotSupported => write!(f, "Service option not supported"),
            Self::ServiceOptionNotSubscribed => {
                write!(f, "Requested service option not subscribed")
            }
            Self::ServiceOptionOutOfOrder => write!(f, "Service option temporarily out of order"),
            Self::NsapiOrPtiAlreadyInUse => write!(f, "NSAPI/PTI already in use"),
            Self::RegularDeactivation => write!(f, "Regular deactivation"),
            Self::QosNotAccepted => write!(f, "QoS not accepted"),
            Self::CallCannotBeIdentified => write!(f, "Call cannot be identified"),
            Self::CsServiceTemporarilyUnavailable => {
                write!(f, "CS service temporarily unavailable")
            }
            Self::FeatureNotSupported => write!(f, "Feature not supported"),
            Self::SemanticErrorInTftOperation => write!(f, "Semantic error in TFT operation"),
            Self::SyntacticalErrorInTftOperation => write!(f, "Syntactical error in TFT operation"),
            Self::UnknownPdpContext => write!(f, "Unknown PDP context"),
            Self::SemanticErrorsInPacketFilter => write!(f, "Semantic error in packet filter"),
            Self::SyntacticalErrorInPacketFilter => write!(f, "Syntactical error in packet filter"),
            Self::PdpContextWithoutTftAlreadyActivated => {
                write!(f, "PDP context without TFT already activated")
            }
            Self::MulticastGroupMembershipTimeout => {
                write!(f, "Multicast group membership timeout")
            }
            Self::GprsUnknown => write!(f, "Unspecified GPRS error"),
            Self::PdpAuthFailure => write!(f, "PDP authentication failure"),
            Self::InvalidMobileClass => write!(f, "Invalid mobile class"),
            Self::LastPdnDisconnectionNotAllowedLegacy => {
                write!(f, "Last PDN disconnection not allowed (legacy)")
            }
            Self::LastPdnDisconnectionNotAllowed => write!(f, "Last PDN disconnection not allowed"),
            Self::SemanticallyIncorrectMessage => write!(f, "Semantically incorrect message"),
            Self::InvalidMandatoryInformation => write!(f, "Invalid mandatory information"),
            Self::MessageTypeNotImplemented => write!(f, "Message type not implemented"),
            Self::ConditionalIeError => write!(f, "Conditional IE error"),
            Self::UnspecifiedProtocolError => write!(f, "Unspecified protocol error"),
            Self::OperatorDeterminedBarring => write!(f, "Operator determined barring"),
            Self::MaximumNumberOfBearersReached => {
                write!(f, "Maximum number of PDP/bearer contexts reached")
            }
            Self::RequestedApnNotSupported => write!(f, "Requested APN not supported"),
            Self::RequestRejectedBcmViolation => write!(f, "Rejected BCM violation"),
            Self::UnsupportedQciOr5QiValue => write!(f, "Unsupported QCI/5QI value"),
            Self::UserDataViaControlPlaneCongested => {
                write!(f, "User data via control plane congested")
            }
            Self::SmsProvidedViaGprsInRoutingArea => {
                write!(f, "SMS provided via GPRS in routing area")
            }
            Self::InvalidPtiValue => write!(f, "Invalid PTI value"),
            Self::NoBearerActivated => write!(f, "No bearer activated"),
            Self::MessageNotCompatibleWithProtocolState => {
                write!(f, "Message not compatible with protocol state")
            }
            Self::RecoveryOnTimerExpiry => write!(f, "Recovery on timer expiry"),
            Self::InvalidTransactionIdValue => write!(f, "Invalid transaction ID value"),
            Self::ServiceOptionNotAuthorizedInPlmn => {
                write!(f, "Service option not authorized in PLMN")
            }
            Self::NetworkFailureActivation => write!(f, "Network failure (activation)"),
            Self::ReactivationRequested => write!(f, "Reactivation requested"),
            Self::Ipv4OnlyAllowed => write!(f, "IPv4 only allowed"),
            Self::Ipv6OnlyAllowed => write!(f, "IPv6 only allowed"),
            Self::SingleAddressBearersOnlyAllowed => {
                write!(f, "Single address bearers only allowed")
            }
            Self::CollisionWithNetworkInitiatedRequest => {
                write!(f, "Collision with network initiated request")
            }
            Self::Ipv4V6OnlyAllowed => write!(f, "IPv4v6 only allowed"),
            Self::NonIpOnlyAllowed => write!(f, "Non-IP only allowed"),
            Self::BearerHandlingUnsupported => write!(f, "Bearer handling unsupported"),
            Self::ApnRestrictionIncompatible => write!(f, "APN restriction incompatible"),
            Self::MultipleAccessToPdnConnectionNotAllowed => {
                write!(f, "Multiple access to PDN connection not allowed")
            }
            Self::EsmInformationNotReceived => write!(f, "ESM information not received"),
            Self::PdnConnectionNonexistent => write!(f, "PDN connection nonexistent"),
            Self::MultiplePdnConnectionSameApnNotAllowed => {
                write!(f, "Multiple PDN connection to same APN not allowed")
            }
            Self::SevereNetworkFailure => write!(f, "Severe network failure"),
            Self::InsufficientResourcesForSliceAndDnn => {
                write!(f, "Insufficient resources for slice and DNN")
            }
            Self::UnsupportedSscMode => write!(f, "Unsupported SSC mode"),
            Self::InsufficientResourcesForSlice => write!(f, "Insufficient resources for slice"),
            Self::MessageTypeNotCompatibleWithProtocolState => {
                write!(f, "Message type not compatible with protocol state")
            }
            Self::IeNotImplemented => write!(f, "IE not implemented"),
            Self::N1ModeNotAllowed => write!(f, "N1 mode not allowed"),
            Self::RestrictedServiceArea => write!(f, "Restricted service area"),
            Self::LadnUnavailable => write!(f, "LADN unavailable"),
            Self::MissingOrUnknownDnnInSlice => write!(f, "Missing or unknown DNN in slice"),
            Self::NgksiAlreadyInUse => write!(f, "ngKSI already in use"),
            Self::PayloadNotForwarded => write!(f, "Payload not forwarded"),
            Self::Non3GppAccessTo5GcnNotAllowed => write!(f, "Non-3GPP access to 5GCN not allowed"),
            Self::ServingNetworkNotAuthorized => write!(f, "Serving network not authorized"),
            Self::DnnNotSupportedInSlice => write!(f, "DNN not supported in slice"),
            Self::InsufficientUserPlaneResourcesForPduSessio => {
                write!(f, "Insufficient user plane resources for PDU session")
            }
            Self::OutOfLadnServiceArea => write!(f, "Out of LADN service area"),
            Self::PtiMismatch => write!(f, "PTI mismatch"),
            Self::MaxDataRateForUserPlaneIntegrityTooLow => {
                write!(f, "Max data rate for user plane integrity too low")
            }
            Self::SemanticErrorInQosOperation => write!(f, "Semantic error in QoS operation"),
            Self::SyntacticalErrorInQosOperation => write!(f, "Syntactical error in QoS operation"),
            Self::InvalidMappedEpsBearerIdentity => write!(f, "Invalid mapped EPS bearer identity"),
            Self::RedirectionTo5GcnRequired => write!(f, "Redirection to 5GCN required"),
            Self::RedirectionToEpcRequired => write!(f, "Redirection to EPC required"),
            Self::TemporarilyUnauthorizedForSnpn => write!(f, "Temporarily unauthorized for SNPN"),
            Self::PermanentlyUnauthorizedForSnpn => write!(f, "Permanently unauthorized for SNPN"),
            Self::EthernetOnlyAllowed => write!(f, "Ethernet only allowed"),
            Self::UnauthorizedForCag => write!(f, "Unauthorized for CAG"),
            Self::NoNetworkSlicesAvailable => write!(f, "No network slices available"),
            Self::WirelineAccessAreaNotAllowed => write!(f, "Wireline access area not allowed"),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for CmeError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::PhoneFailure => defmt::write!(f, "Phone failure"),
            Self::NoConnection => defmt::write!(f, "No connection to phone"),
            Self::LinkReserved => defmt::write!(f, "Phone-adaptor link reserved"),
            Self::NotAllowed => defmt::write!(f, "Operation not allowed"),
            Self::NotSupported => defmt::write!(f, "Operation not supported"),
            Self::PhSimPin => defmt::write!(f, "PH-SIM PIN required"),
            Self::PhFsimPin => defmt::write!(f, "PH-FSIM PIN required"),
            Self::PhFsimPuk => defmt::write!(f, "PH-FSIM PUK required"),
            Self::SimNotInserted => defmt::write!(f, "SIM not inserted"),
            Self::SimPin => defmt::write!(f, "SIM PIN required"),
            Self::SimPuk => defmt::write!(f, "SIM PUK required"),
            Self::SimFailure => defmt::write!(f, "SIM failure"),
            Self::SimBusy => defmt::write!(f, "SIM busy"),
            Self::SimWrong => defmt::write!(f, "SIM wrong"),
            Self::IncorrectPassword => defmt::write!(f, "Incorrect password"),
            Self::SimPin2 => defmt::write!(f, "SIM PIN2 required"),
            Self::SimPuk2 => defmt::write!(f, "SIM PUK2 required"),
            Self::MemoryFull => defmt::write!(f, "Memory full"),
            Self::InvalidIndex => defmt::write!(f, "Invalid index"),
            Self::NotFound => defmt::write!(f, "Not found"),
            Self::MemoryFailure => defmt::write!(f, "Memory failure"),
            Self::TextTooLong => defmt::write!(f, "Text string too long"),
            Self::InvalidChars => defmt::write!(f, "Invalid characters in text string"),
            Self::DialStringTooLong => defmt::write!(f, "Dial string too long"),
            Self::DialStringInvalid => defmt::write!(f, "Invalid characters in dial string"),
            Self::NoNetwork => defmt::write!(f, "No network service"),
            Self::NetworkTimeout => defmt::write!(f, "Network timeout"),
            Self::NetworkNotAllowed => {
                defmt::write!(f, "Network not allowed - emergency calls only")
            }
            Self::NetworkPin => defmt::write!(f, "Network personalization PIN required"),
            Self::NetworkPuk => defmt::write!(f, "Network personalization PUK required"),
            Self::NetworkSubsetPin => {
                defmt::write!(f, "Network subset personalization PIN required")
            }
            Self::NetworkSubsetPuk => {
                defmt::write!(f, "Network subset personalization PUK required")
            }
            Self::ServicePin => defmt::write!(f, "Service provider personalization PIN required"),
            Self::ServicePuk => defmt::write!(f, "Service provider personalization PUK required"),
            Self::CorpPin => defmt::write!(f, "Corporate personalization PIN required"),
            Self::CorpPuk => defmt::write!(f, "Corporate personalization PUK required"),
            Self::HiddenKeyRequired => defmt::write!(f, "Hidden key required"),
            Self::EapMethodNotSupported => defmt::write!(f, "EAP method not supported"),
            Self::IncorrectParameters => defmt::write!(f, "Incorrect parameters"),
            Self::CommandDisabled => defmt::write!(f, "Command disabled"),
            Self::CommandAborted => defmt::write!(f, "Command aborted"),
            Self::NotAttachedRestricted => defmt::write!(f, "Not attached] restricted"),
            Self::NotAllowedEmergencyOnly => defmt::write!(f, "Not allowed] emergency only"),
            Self::NotAllowedRestricted => defmt::write!(f, "Not allowed] restricted"),
            Self::FixedDialNumberOnly => defmt::write!(f, "Fixed dial number only"),
            Self::TemporarilyOutOfService => defmt::write!(f, "Temporarily out of service"),
            Self::LanguageOrAlphabetNotSupported => {
                defmt::write!(f, "Language or alphabet not supported")
            }
            Self::UnexpectedDataValue => defmt::write!(f, "Unexpected data value"),
            Self::SystemFailure => defmt::write!(f, "System failure"),
            Self::DataMissing => defmt::write!(f, "Data missing"),
            Self::CallBarred => defmt::write!(f, "Call barred"),
            Self::MessageWaitingIndicationSubscriptionFailure => {
                defmt::write!(f, "Message waiting indication subscription failure")
            }
            Self::Unknown => defmt::write!(f, "Unknown error"),
            Self::ImsiUnknownInHss => defmt::write!(f, "IMSI unknown in HLR/HSS"),
            Self::IllegalUe => defmt::write!(f, "Illegal MS/UE"),
            Self::ImsiUnknownInVlr => defmt::write!(f, "IMSI unknown in VLR"),
            Self::ImeiNotAccepted => defmt::write!(f, "IMEI not accepted"),
            Self::IllegalMe => defmt::write!(f, "Illegal ME"),
            Self::PsServicesNotAllowed => defmt::write!(f, "PS services not allowed"),
            Self::PsAndNonPsServicesNotAllowed => {
                defmt::write!(f, "PS and non-PS services not allowed")
            }
            Self::UeIdentityNotDerivedFromNetwork => {
                defmt::write!(f, "UE identity not derived from network")
            }
            Self::ImplicitlyDetached => defmt::write!(f, "Implicitly detached"),
            Self::PlmnNotAllowed => defmt::write!(f, "PLMN not allowed"),
            Self::AreaNotAllowed => defmt::write!(f, "Location/tracking area not allowed"),
            Self::RoamingNotAllowedInArea => {
                defmt::write!(f, "Roaming not allowed in this location/tracking area")
            }
            Self::PsServicesNotAllowedInPlmn => defmt::write!(f, "PS services not allowed in PLMN"),
            Self::NoCellsInArea => defmt::write!(f, "No cells in location/tracking area"),
            Self::MscTemporarilyNotReachable => defmt::write!(f, "MSC temporarily not reachable"),
            Self::NetworkFailureAttach => defmt::write!(f, "Network failure (attach)"),
            Self::CsDomainUnavailable => defmt::write!(f, "CS domain unavailable"),
            Self::EsmFailure => defmt::write!(f, "ESM failure"),
            Self::Congestion => defmt::write!(f, "Congestion"),
            Self::MbmsBearerCapabilitiesInsufficientForService => {
                defmt::write!(f, "MBMS bearer capabilities insufficient for service")
            }
            Self::NotAuthorizedForCsg => defmt::write!(f, "Not authorized for CSG"),
            Self::InsufficientResources => defmt::write!(f, "Insufficient resources"),
            Self::MissingOrUnknownApn => defmt::write!(f, "Missing or unknown APN"),
            Self::UnknownPdpAddressOrType => defmt::write!(f, "Unknown PDP address or type"),
            Self::UserAuthenticationFailed => defmt::write!(f, "User authentication failed"),
            Self::ActivationRejectedByGgsnOrGw => {
                defmt::write!(f, "Activation rejected by GGSN or GW")
            }
            Self::ActivationRejectedUnspecified => {
                defmt::write!(f, "Activation rejected (unspecified)")
            }
            Self::ServiceOptionNotSupported => defmt::write!(f, "Service option not supported"),
            Self::ServiceOptionNotSubscribed => {
                defmt::write!(f, "Requested service option not subscribed")
            }
            Self::ServiceOptionOutOfOrder => {
                defmt::write!(f, "Service option temporarily out of order")
            }
            Self::NsapiOrPtiAlreadyInUse => defmt::write!(f, "NSAPI/PTI already in use"),
            Self::RegularDeactivation => defmt::write!(f, "Regular deactivation"),
            Self::QosNotAccepted => defmt::write!(f, "QoS not accepted"),
            Self::CallCannotBeIdentified => defmt::write!(f, "Call cannot be identified"),
            Self::CsServiceTemporarilyUnavailable => {
                defmt::write!(f, "CS service temporarily unavailable")
            }
            Self::FeatureNotSupported => defmt::write!(f, "Feature not supported"),
            Self::SemanticErrorInTftOperation => {
                defmt::write!(f, "Semantic error in TFT operation")
            }
            Self::SyntacticalErrorInTftOperation => {
                defmt::write!(f, "Syntactical error in TFT operation")
            }
            Self::UnknownPdpContext => defmt::write!(f, "Unknown PDP context"),
            Self::SemanticErrorsInPacketFilter => {
                defmt::write!(f, "Semantic error in packet filter")
            }
            Self::SyntacticalErrorInPacketFilter => {
                defmt::write!(f, "Syntactical error in packet filter")
            }
            Self::PdpContextWithoutTftAlreadyActivated => {
                defmt::write!(f, "PDP context without TFT already activated")
            }
            Self::MulticastGroupMembershipTimeout => {
                defmt::write!(f, "Multicast group membership timeout")
            }
            Self::GprsUnknown => defmt::write!(f, "Unspecified GPRS error"),
            Self::PdpAuthFailure => defmt::write!(f, "PDP authentication failure"),
            Self::InvalidMobileClass => defmt::write!(f, "Invalid mobile class"),
            Self::LastPdnDisconnectionNotAllowedLegacy => {
                defmt::write!(f, "Last PDN disconnection not allowed (legacy)")
            }
            Self::LastPdnDisconnectionNotAllowed => {
                defmt::write!(f, "Last PDN disconnection not allowed")
            }
            Self::SemanticallyIncorrectMessage => {
                defmt::write!(f, "Semantically incorrect message")
            }
            Self::InvalidMandatoryInformation => defmt::write!(f, "Invalid mandatory information"),
            Self::MessageTypeNotImplemented => defmt::write!(f, "Message type not implemented"),
            Self::ConditionalIeError => defmt::write!(f, "Conditional IE error"),
            Self::UnspecifiedProtocolError => defmt::write!(f, "Unspecified protocol error"),
            Self::OperatorDeterminedBarring => defmt::write!(f, "Operator determined barring"),
            Self::MaximumNumberOfBearersReached => {
                defmt::write!(f, "Maximum number of PDP/bearer contexts reached")
            }
            Self::RequestedApnNotSupported => defmt::write!(f, "Requested APN not supported"),
            Self::RequestRejectedBcmViolation => defmt::write!(f, "Rejected BCM violation"),
            Self::UnsupportedQciOr5QiValue => defmt::write!(f, "Unsupported QCI/5QI value"),
            Self::UserDataViaControlPlaneCongested => {
                defmt::write!(f, "User data via control plane congested")
            }
            Self::SmsProvidedViaGprsInRoutingArea => {
                defmt::write!(f, "SMS provided via GPRS in routing area")
            }
            Self::InvalidPtiValue => defmt::write!(f, "Invalid PTI value"),
            Self::NoBearerActivated => defmt::write!(f, "No bearer activated"),
            Self::MessageNotCompatibleWithProtocolState => {
                defmt::write!(f, "Message not compatible with protocol state")
            }
            Self::RecoveryOnTimerExpiry => defmt::write!(f, "Recovery on timer expiry"),
            Self::InvalidTransactionIdValue => defmt::write!(f, "Invalid transaction ID value"),
            Self::ServiceOptionNotAuthorizedInPlmn => {
                defmt::write!(f, "Service option not authorized in PLMN")
            }
            Self::NetworkFailureActivation => defmt::write!(f, "Network failure (activation)"),
            Self::ReactivationRequested => defmt::write!(f, "Reactivation requested"),
            Self::Ipv4OnlyAllowed => defmt::write!(f, "IPv4 only allowed"),
            Self::Ipv6OnlyAllowed => defmt::write!(f, "IPv6 only allowed"),
            Self::SingleAddressBearersOnlyAllowed => {
                defmt::write!(f, "Single address bearers only allowed")
            }
            Self::CollisionWithNetworkInitiatedRequest => {
                defmt::write!(f, "Collision with network initiated request")
            }
            Self::Ipv4V6OnlyAllowed => defmt::write!(f, "IPv4v6 only allowed"),
            Self::NonIpOnlyAllowed => defmt::write!(f, "Non-IP only allowed"),
            Self::BearerHandlingUnsupported => defmt::write!(f, "Bearer handling unsupported"),
            Self::ApnRestrictionIncompatible => defmt::write!(f, "APN restriction incompatible"),
            Self::MultipleAccessToPdnConnectionNotAllowed => {
                defmt::write!(f, "Multiple access to PDN connection not allowed")
            }
            Self::EsmInformationNotReceived => defmt::write!(f, "ESM information not received"),
            Self::PdnConnectionNonexistent => defmt::write!(f, "PDN connection nonexistent"),
            Self::MultiplePdnConnectionSameApnNotAllowed => {
                defmt::write!(f, "Multiple PDN connection to same APN not allowed")
            }
            Self::SevereNetworkFailure => defmt::write!(f, "Severe network failure"),
            Self::InsufficientResourcesForSliceAndDnn => {
                defmt::write!(f, "Insufficient resources for slice and DNN")
            }
            Self::UnsupportedSscMode => defmt::write!(f, "Unsupported SSC mode"),
            Self::InsufficientResourcesForSlice => {
                defmt::write!(f, "Insufficient resources for slice")
            }
            Self::MessageTypeNotCompatibleWithProtocolState => {
                defmt::write!(f, "Message type not compatible with protocol state")
            }
            Self::IeNotImplemented => defmt::write!(f, "IE not implemented"),
            Self::N1ModeNotAllowed => defmt::write!(f, "N1 mode not allowed"),
            Self::RestrictedServiceArea => defmt::write!(f, "Restricted service area"),
            Self::LadnUnavailable => defmt::write!(f, "LADN unavailable"),
            Self::MissingOrUnknownDnnInSlice => defmt::write!(f, "Missing or unknown DNN in slice"),
            Self::NgksiAlreadyInUse => defmt::write!(f, "ngKSI already in use"),
            Self::PayloadNotForwarded => defmt::write!(f, "Payload not forwarded"),
            Self::Non3GppAccessTo5GcnNotAllowed => {
                defmt::write!(f, "Non-3GPP access to 5GCN not allowed")
            }
            Self::ServingNetworkNotAuthorized => defmt::write!(f, "Serving network not authorized"),
            Self::DnnNotSupportedInSlice => defmt::write!(f, "DNN not supported in slice"),
            Self::InsufficientUserPlaneResourcesForPduSessio => {
                defmt::write!(f, "Insufficient user plane resources for PDU session")
            }
            Self::OutOfLadnServiceArea => defmt::write!(f, "Out of LADN service area"),
            Self::PtiMismatch => defmt::write!(f, "PTI mismatch"),
            Self::MaxDataRateForUserPlaneIntegrityTooLow => {
                defmt::write!(f, "Max data rate for user plane integrity too low")
            }
            Self::SemanticErrorInQosOperation => {
                defmt::write!(f, "Semantic error in QoS operation")
            }
            Self::SyntacticalErrorInQosOperation => {
                defmt::write!(f, "Syntactical error in QoS operation")
            }
            Self::InvalidMappedEpsBearerIdentity => {
                defmt::write!(f, "Invalid mapped EPS bearer identity")
            }
            Self::RedirectionTo5GcnRequired => defmt::write!(f, "Redirection to 5GCN required"),
            Self::RedirectionToEpcRequired => defmt::write!(f, "Redirection to EPC required"),
            Self::TemporarilyUnauthorizedForSnpn => {
                defmt::write!(f, "Temporarily unauthorized for SNPN")
            }
            Self::PermanentlyUnauthorizedForSnpn => {
                defmt::write!(f, "Permanently unauthorized for SNPN")
            }
            Self::EthernetOnlyAllowed => defmt::write!(f, "Ethernet only allowed"),
            Self::UnauthorizedForCag => defmt::write!(f, "Unauthorized for CAG"),
            Self::NoNetworkSlicesAvailable => defmt::write!(f, "No network slices available"),
            Self::WirelineAccessAreaNotAllowed => {
                defmt::write!(f, "Wireline access area not allowed")
            }
        }
    }
}
