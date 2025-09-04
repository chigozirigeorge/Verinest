// use web3::{
//     signing::{recover, Signature}, types::{self, Recovery, H160}
// };
// use hex;
// use tiny_keccak::{Keccak, Hasher};
// use std::str::FromStr;

// pub struct WalletVerificationService;

// impl WalletVerificationService {
//      //Generate a verification message for the user to sign
//     pub fn generate_verification_message(
//         user_id: &str,
//         nonce: &str,
//     ) -> String {
//         format!("Please sign this message to verify your wallet ownership.\n\nUser ID: {}\nNonce: {}", user_id, nonce)
//     }

//     //Generate a nonce for verification
//     pub fn generate_nonce() -> String {
//         use rand::Rng;
//         let mut rng = rand::rng();
//         format!("{:06}", rng.random_range(100000..999999))
//     }

//     //Keccak-256 hash function
//     fn keccak256(data: &[u8]) -> [u8; 32] {
//         let mut keccak = Keccak::v256();
//         let mut output = [0u8; 32];
//         keccak.update(data);
//         keccak.finalize(&mut output);
//         output
//     }

//     //EIP-191 hash function
//     fn eip191_hash(message: &str) -> [u8; 32] {
//         let prefix = "\x19Ethereum Signed Message:\n";
//         let message_len = message.len();
//         let mut data = Vec::new();
//         data.extend_from_slice(prefix.as_bytes());
//         data.extend_from_slice(message_len.to_string().as_bytes());
//         data.extend_from_slice(message.as_bytes());
//         Self::keccak256(&data)
//     }

//     //validate Ethereum address format
//     pub fn is_valid_ethereum_address(address: &str) -> bool {
//         let clean_address = address.trim_start_matches("0x");
//         if clean_address.len() != 40 {
//             return false
//         }
//         clean_address.chars().all(|c| c.is_ascii_hexdigit())
//     }

//     //verify an Ethereum wallet signature
//     pub fn verify_ethereum_signature(
//         message: &str,
//         signature: &str,
//         expected_address: &str,
//     ) -> Result<bool, String> {
//         //Prepare the message with Ethereum prefix
//         let prefixed_message = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);

//         //Hash the prefixed message
//         let message_hash = Self::keccak256(prefixed_message.as_bytes());

//         //Parse the signature
//         let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
//             .map_err(|e| format!("Invalid signature format: {}", e))?;

//         if signature_bytes.len() != 65 {
//             return  Err("Signature must be 65 bytes long".to_string());
//         }

//         //Extract v, r, s from the signature
//         let v = signature_bytes[64].into();
//         let r = &signature_bytes[0..32];
//         let s = &signature_bytes[32..64];

//         //We recover the address
//         let recovery = Recovery::new(
//             message_hash, Signature::from_rsv(
//             r.try_into().map_err(|_| "invalid r value")?, 
//             s.try_into().map_err(|_| "invalid s value")?,
//             v,
//         ));

//         let recovered_address = recover(None, &recovery, None)
//         .map_err(|e| format!("Failed to recover address: {}", e))?;

//         //parse expected address
//         let expected_address = H160::from_str(expected_address.trim_start_matches("0x"))
//             .map_err(|e| format!("Invalid address format: {}", e))?;

//         //compare addresses (case -insensitive)
//         Ok(recovered_address.to_low_u64_be() == expected_address.to_low_u64_be())
//     }

//     //verify a generic EIP-191 signature
//     pub fn verify_eip191_signature(
//         message: &str,
//         signature: &str,
//         expected_address: &str
//     ) -> Result<bool, String> {
//         let message_hash = Self::eip191_hash(message);

//         let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
//             .map_err(|e| format!("Invalid signature format: {}", e))?;

//         if signature_bytes.len() != 65 {
//             return Err("Signature must be 65 bytes long".to_string());
//         }

//         let v = signature_bytes[64].into();
//         let r = &signature_bytes[0..32];
//         let s = &signature_bytes[32..64];

//         let recovery = Recovery::new(message_hash,
//             v, 
//             &r.try_into().map_err(|_| "Invalid r value")?, 
//             &s.try_into().map_err(|_| "Invalid s value")?,
//         );

//         let recovered_address = recover(None, &recovery, None)
//             .map_err(|e| format!("Signature Recovery failed: {}", e))?;

//         let expected_address = H160::from_str(expected_address.trim_start_matches("0x"))
//             .map_err(|e| format!("Invalid address format: {}", e))?;

//         Ok(recovered_address == expected_address)
//     }
// }