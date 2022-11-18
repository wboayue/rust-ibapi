use crate::server_versions;

// 100+ messaging */
// 100 = enhanced handshake, msg length prefixes

pub const CLIENT_VERSION: i32 = 66;  //API v. 9.71

pub const MIN_CLIENT_VER: i32 = 100;
pub const MAX_CLIENT_VER: i32 = server_versions::PRICE_MGMT_ALGO;

// public const int ClientVersion = 66;//API v. 9.71
// public const byte EOL = 0;
// public const string BagSecType = "BAG";
// public const int REDIRECT_COUNT_MAX = 2;
// public const string INFINITY_STR = "Infinity";

// public const int FaGroups = 1;
// public const int FaProfiles = 2;
// public const int FaAliases = 3;
// public const int MinVersion = 100;
// public const int MaxVersion = MinServerVer.MIN_SERVER_VER_BOND_ISSUERID;
// public const int MaxMsgSize = 0x00FFFFFF;
