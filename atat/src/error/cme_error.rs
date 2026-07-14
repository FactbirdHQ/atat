/// Enumeration of Mobile Equipment errors, as defined in 3GPP TS 27.007
/// v17.1.0, section 9.2 (Mobile termination error result code +CME ERROR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmeError {
    /// nick=PhoneFailure
    PhoneFailure,
    /// nick=NoConnection
    NoConnection,
    /// nick=LinkReserved
    LinkReserved,
    /// nick=NotAllowed
    NotAllowed,
    /// nick=NotSupported
    NotSupported,
    /// nick=PhSimPin
    PhSimPin,
    /// nick=PhFsimPin
    PhFsimPin,
    /// nick=PhFsimPuk
    PhFsimPuk,
    /// nick=SimNotInserted
    SimNotInserted,
    /// nick=SimPin
    SimPin,
    /// nick=SimPuk
    SimPuk,
    /// nick=SimFailure
    SimFailure,
    /// nick=SimBusy
    SimBusy,
    /// nick=SimWrong
    SimWrong,
    /// nick=IncorrectPassword
    IncorrectPassword,
    /// nick=SimPin2
    SimPin2,
    /// nick=SimPuk2
    SimPuk2,
    /// nick=MemoryFull
    MemoryFull,
    /// nick=InvalidIndex
    InvalidIndex,
    /// nick=NotFound
    NotFound,
    /// nick=MemoryFailure
    MemoryFailure,
    /// nick=TextTooLong
    TextTooLong,
    /// nick=InvalidChars
    InvalidChars,
    /// nick=DialStringTooLong
    DialStringTooLong,
    /// nick=DialStringInvalid
    DialStringInvalid,
    /// nick=NoNetwork
    NoNetwork,
    /// nick=NetworkTimeout
    NetworkTimeout,
    /// nick=NetworkNotAllowed
    NetworkNotAllowed,
    /// nick=NetworkPin
    NetworkPin,
    /// nick=NetworkPuk
    NetworkPuk,
    /// nick=NetworkSubsetPin
    NetworkSubsetPin,
    /// nick=NetworkSubsetPuk
    NetworkSubsetPuk,
    /// nick=ServicePin
    ServicePin,
    /// nick=ServicePuk
    ServicePuk,
    /// nick=CorpPin
    CorpPin,
    /// nick=CorpPuk
    CorpPuk,
    /// nick=HiddenKeyRequired
    HiddenKeyRequired,
    /// nick=EapMethodNotSupported
    EapMethodNotSupported,
    /// nick=IncorrectParameters
    IncorrectParameters,
    /// nick=CommandDisabled
    CommandDisabled,
    /// nick=CommandAborted
    CommandAborted,
    /// nick=NotAttachedRestricted
    NotAttachedRestricted,
    /// nick=NotAllowedEmergencyOnly
    NotAllowedEmergencyOnly,
    /// nick=NotAllowedRestricted
    NotAllowedRestricted,
    /// nick=FixedDialNumberOnly
    FixedDialNumberOnly,
    /// nick=TemporarilyOutOfService
    TemporarilyOutOfService,
    /// nick=LanguageOrAlphabetNotSupported
    LanguageOrAlphabetNotSupported,
    /// nick=UnexpectedDataValue
    UnexpectedDataValue,
    /// nick=SystemFailure
    SystemFailure,
    /// nick=DataMissing
    DataMissing,
    /// nick=CallBarred
    CallBarred,
    /// nick=MessageWaitingIndicationSubscriptionFailure
    MessageWaitingIndicationSubscriptionFailure,
    /// nick=Unknown
    Unknown,
    /// nick=ImsiUnknownInHss
    ImsiUnknownInHss,
    /// nick=IllegalUe
    IllegalUe,
    /// nick=ImsiUnknownInVlr
    ImsiUnknownInVlr,
    /// nick=ImeiNotAccepted
    ImeiNotAccepted,
    /// nick=IllegalMe
    IllegalMe,
    /// nick=PsServicesNotAllowed
    PsServicesNotAllowed,
    /// nick=PsAndNonPsServicesNotAllowed
    PsAndNonPsServicesNotAllowed,
    /// nick=UeIdentityNotDerivedFromNetwork
    UeIdentityNotDerivedFromNetwork,
    /// nick=ImplicitlyDetached
    ImplicitlyDetached,
    /// nick=PlmnNotAllowed
    PlmnNotAllowed,
    /// nick=AreaNotAllowed
    AreaNotAllowed,
    /// nick=RoamingNotAllowedInArea
    RoamingNotAllowedInArea,
    /// nick=PsServicesNotAllowedInPlmn
    PsServicesNotAllowedInPlmn,
    /// nick=NoCellsInArea
    NoCellsInArea,
    /// nick=MscTemporarilyNotReachable
    MscTemporarilyNotReachable,
    /// nick=NetworkFailureAttach
    NetworkFailureAttach,
    /// nick=CsDomainUnavailable
    CsDomainUnavailable,
    /// nick=EsmFailure
    EsmFailure,
    /// nick=Congestion
    Congestion,
    /// nick=MbmsBearerCapabilitiesInsufficientForService
    MbmsBearerCapabilitiesInsufficientForService,
    /// nick=NotAuthorizedForCsg
    NotAuthorizedForCsg,
    /// nick=InsufficientResources
    InsufficientResources,
    /// nick=MissingOrUnknownApn
    MissingOrUnknownApn,
    /// nick=UnknownPdpAddressOrType
    UnknownPdpAddressOrType,
    /// nick=UserAuthenticationFailed
    UserAuthenticationFailed,
    /// nick=ActivationRejectedByGgsnOrGw
    ActivationRejectedByGgsnOrGw,
    /// nick=ActivationRejectedUnspecified
    ActivationRejectedUnspecified,
    /// nick=ServiceOptionNotSupported
    ServiceOptionNotSupported,
    /// nick=ServiceOptionNotSubscribed
    ServiceOptionNotSubscribed,
    /// nick=ServiceOptionOutOfOrder
    ServiceOptionOutOfOrder,
    /// nick=NsapiOrPtiAlreadyInUse
    NsapiOrPtiAlreadyInUse,
    /// nick=RegularDeactivation
    RegularDeactivation,
    /// nick=QosNotAccepted
    QosNotAccepted,
    /// nick=CallCannotBeIdentified
    CallCannotBeIdentified,
    /// nick=CsServiceTemporarilyUnavailable
    CsServiceTemporarilyUnavailable,
    /// nick=FeatureNotSupported
    FeatureNotSupported,
    /// nick=SemanticErrorInTftOperation
    SemanticErrorInTftOperation,
    /// nick=SyntacticalErrorInTftOperation
    SyntacticalErrorInTftOperation,
    /// nick=UnknownPdpContext
    UnknownPdpContext,
    /// nick=SemanticErrorsInPacketFilter
    SemanticErrorsInPacketFilter,
    /// nick=SyntacticalErrorsInPacketFilter
    SyntacticalErrorInPacketFilter,
    /// nick=PdpContextWithoutTftAlreadyActivated
    PdpContextWithoutTftAlreadyActivated,
    /// nick=MulticastGroupMembershipTimeout
    MulticastGroupMembershipTimeout,
    /// nick=GprsUnknown
    GprsUnknown,
    /// nick=PdpAuthFailure
    PdpAuthFailure,
    /// nick=InvalidMobileClass
    InvalidMobileClass,
    /// nick=LastPdnDisconnectionNotAllowedLegacy
    LastPdnDisconnectionNotAllowedLegacy,
    /// nick=LastPdnDisconnectionNotAllowed
    LastPdnDisconnectionNotAllowed,
    /// nick=SemanticallyIncorrectMessage
    SemanticallyIncorrectMessage,
    /// nick=InvalidMandatoryInformation
    InvalidMandatoryInformation,
    /// nick=MessageTypeNotImplemented
    MessageTypeNotImplemented,
    /// nick=ConditionalIeError
    ConditionalIeError,
    /// nick=UnspecifiedProtocolError
    UnspecifiedProtocolError,
    /// nick=OperatorDeterminedBarring
    OperatorDeterminedBarring,
    /// nick=MaximumNumberOfBearersReached
    MaximumNumberOfBearersReached,
    /// nick=RequestedApnNotSupported
    RequestedApnNotSupported,
    /// nick=RequestRejectedBcmViolation
    RequestRejectedBcmViolation,
    /// nick=UnsupportedQciOr5qiValue
    UnsupportedQciOr5QiValue,
    /// nick=UserDataViaControlPlaneCongested
    UserDataViaControlPlaneCongested,
    /// nick=SmsProvidedViaGprsInRoutingArea
    SmsProvidedViaGprsInRoutingArea,
    /// nick=InvalidPtiValue
    InvalidPtiValue,
    /// nick=NoBearerActivated
    NoBearerActivated,
    /// nick=MessageNotCompatibleWithProtocolState
    MessageNotCompatibleWithProtocolState,
    /// nick=RecoveryOnTimerExpiry
    RecoveryOnTimerExpiry,
    /// nick=InvalidTransactionIdValue
    InvalidTransactionIdValue,
    /// nick=ServiceOptionNotAuthorizedInPlmn
    ServiceOptionNotAuthorizedInPlmn,
    /// nick=NetworkFailureActivation
    NetworkFailureActivation,
    /// nick=ReactivationRequested
    ReactivationRequested,
    /// nick=Ipv4OnlyAllowed
    Ipv4OnlyAllowed,
    /// nick=Ipv6OnlyAllowed
    Ipv6OnlyAllowed,
    /// nick=SingleAddressBearersOnlyAllowed
    SingleAddressBearersOnlyAllowed,
    /// nick=CollisionWithNetworkInitiatedRequest
    CollisionWithNetworkInitiatedRequest,
    /// nick=Ipv4v6OnlyAllowed
    Ipv4V6OnlyAllowed,
    /// nick=NonIpOnlyAllowed
    NonIpOnlyAllowed,
    /// nick=BearerHandlingUnsupported
    BearerHandlingUnsupported,
    /// nick=ApnRestrictionIncompatible
    ApnRestrictionIncompatible,
    /// nick=MultipleAccessToPdnConnectionNotAllowed
    MultipleAccessToPdnConnectionNotAllowed,
    /// nick=EsmInformationNotReceived
    EsmInformationNotReceived,
    /// nick=PdnConnectionNonexistent
    PdnConnectionNonexistent,
    /// nick=MultiplePdnConnectionSameApnNotAllowed
    MultiplePdnConnectionSameApnNotAllowed,
    /// nick=SevereNetworkFailure
    SevereNetworkFailure,
    /// nick=InsufficientResourcesForSliceAndDnn
    InsufficientResourcesForSliceAndDnn,
    /// nick=UnsupportedSscMode
    UnsupportedSscMode,
    /// nick=InsufficientResourcesForSlice
    InsufficientResourcesForSlice,
    /// nick=MessageTypeNotCompatibleWithProtocolState
    MessageTypeNotCompatibleWithProtocolState,
    /// nick=IeNotImplemented
    IeNotImplemented,
    /// nick=N1ModeNotAllowed
    N1ModeNotAllowed,
    /// nick=RestrictedServiceArea
    RestrictedServiceArea,
    /// nick=LadnUnavailable
    LadnUnavailable,
    /// nick=MissingOrUnknownDnnInSlice
    MissingOrUnknownDnnInSlice,
    /// nick=NkgsiAlreadyInUse
    NgksiAlreadyInUse,
    /// nick=PayloadNotForwarded
    PayloadNotForwarded,
    /// nick=Non3gppAccessTo5gcnNotAllowed
    Non3GppAccessTo5GcnNotAllowed,
    /// nick=ServingNetworkNotAuthorized
    ServingNetworkNotAuthorized,
    /// nick=DnnNotSupportedInSlice
    DnnNotSupportedInSlice,
    /// nick=InsufficientUserPlaneResourcesForPduSession
    InsufficientUserPlaneResourcesForPduSessio,
    /// nick=OutOfLadnServiceArea
    OutOfLadnServiceArea,
    /// nick=PtiMismatch
    PtiMismatch,
    /// nick=MaxDataRateForUserPlaneIntegrityTooLow
    MaxDataRateForUserPlaneIntegrityTooLow,
    /// nick=SemanticErrorInQosOperation
    SemanticErrorInQosOperation,
    /// nick=SyntacticalErrorInQosOperation
    SyntacticalErrorInQosOperation,
    /// nick=InvalidMappedEpsBearerIdentity
    InvalidMappedEpsBearerIdentity,
    /// nick=RedirectionTo5gcnRequired
    RedirectionTo5GcnRequired,
    /// nick=RedirectionToEpcRequired
    RedirectionToEpcRequired,
    /// nick=TemporarilyUnauthorizedForSnpn
    TemporarilyUnauthorizedForSnpn,
    /// nick=PermanentlyUnauthorizedForSnpn
    PermanentlyUnauthorizedForSnpn,
    /// nick=EthernetOnlyAllowed
    EthernetOnlyAllowed,
    /// nick=UnauthorizedForCag
    UnauthorizedForCag,
    /// nick=NoNetworkSlicesAvailable
    NoNetworkSlicesAvailable,
    /// nick=WirelineAccessAreaNotAllowed
    WirelineAccessAreaNotAllowed,
    /// All values below 256 are reserved.
    Reserved(u16),
    /// All values above and including 256 are manufactuer specific.
    ManufacturerSpecific(u16),
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
            100 => Self::Unknown,
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
            0..256 => Self::Reserved(v),
            256.. => Self::ManufacturerSpecific(v),
        }
    }
}

impl From<CmeError> for u16 {
    fn from(error: CmeError) -> Self {
        match error {
            CmeError::PhoneFailure => 0,
            CmeError::NoConnection => 1,
            CmeError::LinkReserved => 2,
            CmeError::NotAllowed => 3,
            CmeError::NotSupported => 4,
            CmeError::PhSimPin => 5,
            CmeError::PhFsimPin => 6,
            CmeError::PhFsimPuk => 7,
            CmeError::SimNotInserted => 10,
            CmeError::SimPin => 11,
            CmeError::SimPuk => 12,
            CmeError::SimFailure => 13,
            CmeError::SimBusy => 14,
            CmeError::SimWrong => 15,
            CmeError::IncorrectPassword => 16,
            CmeError::SimPin2 => 17,
            CmeError::SimPuk2 => 18,
            CmeError::MemoryFull => 20,
            CmeError::InvalidIndex => 21,
            CmeError::NotFound => 22,
            CmeError::MemoryFailure => 23,
            CmeError::TextTooLong => 24,
            CmeError::InvalidChars => 25,
            CmeError::DialStringTooLong => 26,
            CmeError::DialStringInvalid => 27,
            CmeError::NoNetwork => 30,
            CmeError::NetworkTimeout => 31,
            CmeError::NetworkNotAllowed => 32,
            CmeError::NetworkPin => 40,
            CmeError::NetworkPuk => 41,
            CmeError::NetworkSubsetPin => 42,
            CmeError::NetworkSubsetPuk => 43,
            CmeError::ServicePin => 44,
            CmeError::ServicePuk => 45,
            CmeError::CorpPin => 46,
            CmeError::CorpPuk => 47,
            CmeError::HiddenKeyRequired => 48,
            CmeError::EapMethodNotSupported => 49,
            CmeError::IncorrectParameters => 50,
            CmeError::CommandDisabled => 51,
            CmeError::CommandAborted => 52,
            CmeError::NotAttachedRestricted => 53,
            CmeError::NotAllowedEmergencyOnly => 54,
            CmeError::NotAllowedRestricted => 55,
            CmeError::FixedDialNumberOnly => 56,
            CmeError::TemporarilyOutOfService => 57,
            CmeError::LanguageOrAlphabetNotSupported => 58,
            CmeError::UnexpectedDataValue => 59,
            CmeError::SystemFailure => 60,
            CmeError::DataMissing => 61,
            CmeError::CallBarred => 62,
            CmeError::MessageWaitingIndicationSubscriptionFailure => 63,
            CmeError::Unknown => 100,
            CmeError::ImsiUnknownInHss => 102,
            CmeError::IllegalUe => 103,
            CmeError::ImsiUnknownInVlr => 104,
            CmeError::ImeiNotAccepted => 105,
            CmeError::IllegalMe => 106,
            CmeError::PsServicesNotAllowed => 107,
            CmeError::PsAndNonPsServicesNotAllowed => 108,
            CmeError::UeIdentityNotDerivedFromNetwork => 109,
            CmeError::ImplicitlyDetached => 110,
            CmeError::PlmnNotAllowed => 111,
            CmeError::AreaNotAllowed => 112,
            CmeError::RoamingNotAllowedInArea => 113,
            CmeError::PsServicesNotAllowedInPlmn => 114,
            CmeError::NoCellsInArea => 115,
            CmeError::MscTemporarilyNotReachable => 116,
            CmeError::NetworkFailureAttach => 117,
            CmeError::CsDomainUnavailable => 118,
            CmeError::EsmFailure => 119,
            CmeError::Congestion => 122,
            CmeError::MbmsBearerCapabilitiesInsufficientForService => 124,
            CmeError::NotAuthorizedForCsg => 125,
            CmeError::InsufficientResources => 126,
            CmeError::MissingOrUnknownApn => 127,
            CmeError::UnknownPdpAddressOrType => 128,
            CmeError::UserAuthenticationFailed => 129,
            CmeError::ActivationRejectedByGgsnOrGw => 130,
            CmeError::ActivationRejectedUnspecified => 131,
            CmeError::ServiceOptionNotSupported => 132,
            CmeError::ServiceOptionNotSubscribed => 133,
            CmeError::ServiceOptionOutOfOrder => 134,
            CmeError::NsapiOrPtiAlreadyInUse => 135,
            CmeError::RegularDeactivation => 136,
            CmeError::QosNotAccepted => 137,
            CmeError::CallCannotBeIdentified => 138,
            CmeError::CsServiceTemporarilyUnavailable => 139,
            CmeError::FeatureNotSupported => 140,
            CmeError::SemanticErrorInTftOperation => 141,
            CmeError::SyntacticalErrorInTftOperation => 142,
            CmeError::UnknownPdpContext => 143,
            CmeError::SemanticErrorsInPacketFilter => 144,
            CmeError::SyntacticalErrorInPacketFilter => 145,
            CmeError::PdpContextWithoutTftAlreadyActivated => 146,
            CmeError::MulticastGroupMembershipTimeout => 147,
            CmeError::GprsUnknown => 148,
            CmeError::PdpAuthFailure => 149,
            CmeError::InvalidMobileClass => 150,
            CmeError::LastPdnDisconnectionNotAllowedLegacy => 151,
            CmeError::LastPdnDisconnectionNotAllowed => 171,
            CmeError::SemanticallyIncorrectMessage => 172,
            CmeError::InvalidMandatoryInformation => 173,
            CmeError::MessageTypeNotImplemented => 174,
            CmeError::ConditionalIeError => 175,
            CmeError::UnspecifiedProtocolError => 176,
            CmeError::OperatorDeterminedBarring => 177,
            CmeError::MaximumNumberOfBearersReached => 178,
            CmeError::RequestedApnNotSupported => 179,
            CmeError::RequestRejectedBcmViolation => 180,
            CmeError::UnsupportedQciOr5QiValue => 181,
            CmeError::UserDataViaControlPlaneCongested => 182,
            CmeError::SmsProvidedViaGprsInRoutingArea => 183,
            CmeError::InvalidPtiValue => 184,
            CmeError::NoBearerActivated => 185,
            CmeError::MessageNotCompatibleWithProtocolState => 186,
            CmeError::RecoveryOnTimerExpiry => 187,
            CmeError::InvalidTransactionIdValue => 188,
            CmeError::ServiceOptionNotAuthorizedInPlmn => 189,
            CmeError::NetworkFailureActivation => 190,
            CmeError::ReactivationRequested => 191,
            CmeError::Ipv4OnlyAllowed => 192,
            CmeError::Ipv6OnlyAllowed => 193,
            CmeError::SingleAddressBearersOnlyAllowed => 194,
            CmeError::CollisionWithNetworkInitiatedRequest => 195,
            CmeError::Ipv4V6OnlyAllowed => 196,
            CmeError::NonIpOnlyAllowed => 197,
            CmeError::BearerHandlingUnsupported => 198,
            CmeError::ApnRestrictionIncompatible => 199,
            CmeError::MultipleAccessToPdnConnectionNotAllowed => 200,
            CmeError::EsmInformationNotReceived => 201,
            CmeError::PdnConnectionNonexistent => 202,
            CmeError::MultiplePdnConnectionSameApnNotAllowed => 203,
            CmeError::SevereNetworkFailure => 204,
            CmeError::InsufficientResourcesForSliceAndDnn => 205,
            CmeError::UnsupportedSscMode => 206,
            CmeError::InsufficientResourcesForSlice => 207,
            CmeError::MessageTypeNotCompatibleWithProtocolState => 208,
            CmeError::IeNotImplemented => 209,
            CmeError::N1ModeNotAllowed => 210,
            CmeError::RestrictedServiceArea => 211,
            CmeError::LadnUnavailable => 212,
            CmeError::MissingOrUnknownDnnInSlice => 213,
            CmeError::NgksiAlreadyInUse => 214,
            CmeError::PayloadNotForwarded => 215,
            CmeError::Non3GppAccessTo5GcnNotAllowed => 216,
            CmeError::ServingNetworkNotAuthorized => 217,
            CmeError::DnnNotSupportedInSlice => 218,
            CmeError::InsufficientUserPlaneResourcesForPduSessio => 219,
            CmeError::OutOfLadnServiceArea => 220,
            CmeError::PtiMismatch => 221,
            CmeError::MaxDataRateForUserPlaneIntegrityTooLow => 222,
            CmeError::SemanticErrorInQosOperation => 223,
            CmeError::SyntacticalErrorInQosOperation => 224,
            CmeError::InvalidMappedEpsBearerIdentity => 225,
            CmeError::RedirectionTo5GcnRequired => 226,
            CmeError::RedirectionToEpcRequired => 227,
            CmeError::TemporarilyUnauthorizedForSnpn => 228,
            CmeError::PermanentlyUnauthorizedForSnpn => 229,
            CmeError::EthernetOnlyAllowed => 230,
            CmeError::UnauthorizedForCag => 231,
            CmeError::NoNetworkSlicesAvailable => 232,
            CmeError::WirelineAccessAreaNotAllowed => 233,
            CmeError::Reserved(error) | CmeError::ManufacturerSpecific(error) => error,
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
            Self::Reserved(error) => write!(f, "Unknown reserved error {error}"),
            Self::ManufacturerSpecific(error) => write!(f, "Manufacturer specific error {error}"),
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
            Self::Reserved(error) => defmt::write!(f, "Unknown reserved error {}", error),
            Self::ManufacturerSpecific(error) => {
                defmt::write!(f, "Manufacturer specific error {}", error)
            }
        }
    }
}
