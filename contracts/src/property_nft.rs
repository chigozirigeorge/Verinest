use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Bytes, Env, Map, String, Symblo, Vec
};

#[derive(Clone)]
#[contracttype]
pub struct PropertyMetadata{
    pub property_id: String,
    pub title: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub property_type: String,
    pub listing_type: String,
    pub price: i128,
    pub currency: String,
    pub coordinates: Option<(i64, i64)>,
    pub bathrooms: Option<u32>,
    pub bathrooms: Option<u32>,
    pub size_sqm: Option<u64>,
    pub landlord_id: String,
    pub agent_id: Option<String>,
    pub lawyer_id: Option<String>,
    pub certificate_hash: Bytes,
    pub verification_status: String,
    pub created_at: u64,
    pub verified_at: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct VerificationRecord {
    pub verifier_id: String,
    pub verifier_type: String,
    pub status: String,
    pub notes: String,
    pub verification_hash: Bytes,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    TokenCounter,
    TokenMetadata(u64),
    TokenOwner(u64),
    OwnerTokens(Address),
    TokenApproval(u64),
    OperatorApproval(Address, Address),
    PropertyVerifications(u64),
    Admin,
    Name,
    Symbol,
}

#[contract]
pub struct PropertyNFTContract;

#[contractimpl]
impl PropertyNFTContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
    ) {
        if env.storage().persistent().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        admin.require_auth();

        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Name, &name);
        env.storage().persistent().set(&DataKey::Symbol, &symbol);
        env.storage().persistent().set(&DataKey::TokenCounter,&0u64);
    }

    //Mint a new property NFT (only admin can mint)
    pub fn mint_property_nft(
        env: Env,
        to: Address,
        property_metadata: PropertyMetadata,
    ) -> u64 {
        let admin: Address = env
    }
}