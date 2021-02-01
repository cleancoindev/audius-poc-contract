#![cfg(feature = "test-bpf")]

use audius::*;
use rand::{thread_rng, Rng};
use secp256k1::{Message, PublicKey, RecoveryId, SecretKey, Signature};
use sha3::Digest;
use solana_program::{hash::Hash, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use std::mem::size_of;

pub fn program_test() -> ProgramTest {
    ProgramTest::new("audius", id(), processor!(processor::Processor::process))
}

async fn setup() -> (BanksClient, Keypair, Hash, Keypair, Keypair) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let signer_group = Keypair::new();
    let group_owner = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &signer_group,
        state::SignerGroup::LEN,
    )
    .await
    .unwrap();

    (
        banks_client,
        payer,
        recent_blockhash,
        signer_group,
        group_owner,
    )
}

async fn create_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    struct_size: usize,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(struct_size);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            account_rent,
            struct_size as u64,
            &id(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> Account {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

async fn process_tx_init_signer_group(
    signer_group: &Pubkey,
    group_owner: &Pubkey,
    payer: &Keypair,
    recent_blockhash: Hash,
    banks_client: &mut BanksClient,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::init_signer_group(&id(), signer_group, group_owner).unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn process_tx_init_valid_signer(
    valid_signer: &Pubkey,
    signer_group: &Pubkey,
    group_owner: &Keypair,
    payer: &Keypair,
    recent_blockhash: Hash,
    banks_client: &mut BanksClient,
    eth_pub_key: [u8; 20],
) -> Result<(), TransportError> {
    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::init_valid_signer(
            &id(),
            valid_signer,
            signer_group,
            &group_owner.pubkey(),
            eth_pub_key,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, group_owner], latest_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

fn construct_eth_pubkey(pubkey: &PublicKey) -> [u8; 20] {
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&sha3::Keccak256::digest(&pubkey.serialize()[1..])[12..]);
    assert_eq!(addr.len(), 20);
    addr
}

#[tokio::test]
async fn init_signer_group() {
    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let signer_group_account = get_account(&mut banks_client, &signer_group.pubkey()).await;

    assert_eq!(signer_group_account.data.len(), state::SignerGroup::LEN);
    assert_eq!(signer_group_account.owner, id());

    let signer_group_data =
        state::SignerGroup::deserialize(&signer_group_account.data.as_slice()).unwrap();

    assert!(signer_group_data.is_initialized());
    assert_eq!(signer_group_data.owner, group_owner.pubkey());
}

#[tokio::test]
async fn init_valid_signer() {
    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let valid_signer = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &valid_signer,
        state::ValidSigner::LEN,
    )
    .await
    .unwrap();

    let eth_pub_key = [1u8; 20];
    process_tx_init_valid_signer(
        &valid_signer.pubkey(),
        &signer_group.pubkey(),
        &group_owner,
        &payer,
        recent_blockhash,
        &mut banks_client,
        eth_pub_key,
    )
    .await
    .unwrap();

    let valid_signer_account = get_account(&mut banks_client, &valid_signer.pubkey()).await;

    assert_eq!(valid_signer_account.data.len(), state::ValidSigner::LEN);
    assert_eq!(valid_signer_account.owner, id());

    let valid_signer_data =
        state::ValidSigner::deserialize(&valid_signer_account.data.as_slice()).unwrap();

    assert!(valid_signer_data.is_initialized());
    assert_eq!(valid_signer_data.public_key, eth_pub_key);
    assert_eq!(valid_signer_data.signer_group, signer_group.pubkey());
}

#[tokio::test]
async fn clear_valid_signer() {
    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let valid_signer = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &valid_signer,
        state::ValidSigner::LEN,
    )
    .await
    .unwrap();

    let eth_pub_key = [1u8; 20];
    process_tx_init_valid_signer(
        &valid_signer.pubkey(),
        &signer_group.pubkey(),
        &group_owner,
        &payer,
        recent_blockhash,
        &mut banks_client,
        eth_pub_key,
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::clear_valid_signer(
            &id(),
            &valid_signer.pubkey(),
            &signer_group.pubkey(),
            &group_owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &group_owner], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let valid_signer_account = get_account(&mut banks_client, &valid_signer.pubkey()).await;

    let valid_signer_data =
        state::ValidSigner::deserialize(&valid_signer_account.data.as_slice()).unwrap();

    assert_eq!(valid_signer_data.is_initialized(), false);
}

#[tokio::test]
async fn validate_signature() {
    let mut rng = thread_rng();
    let key: [u8; 32] = rng.gen();
    let priv_key = SecretKey::parse(&key).unwrap();
    let secp_pubkey = PublicKey::from_secret_key(&priv_key);
    let eth_pubkey = construct_eth_pubkey(&secp_pubkey);

    let message = vec![5; 20];

    let mut hasher = sha3::Keccak256::new();
    hasher.update(&message);

    let message_hash = hasher.finalize();
    let mut message_hash_arr = [0u8; 32];
    message_hash_arr.copy_from_slice(&message_hash.as_slice());
    let message = Message::parse(&message_hash_arr);
    let (signature, recovery_id) = secp256k1::sign(&message, &priv_key);
    let signature_arr = signature.serialize();

    let signature_param = instruction::Signature {
        signature: signature_arr,
        recovery_id: recovery_id.serialize(),
        message: message_hash_arr,
    };

    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let valid_signer = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &valid_signer,
        state::ValidSigner::LEN,
    )
    .await
    .unwrap();

    process_tx_init_valid_signer(
        &valid_signer.pubkey(),
        &signer_group.pubkey(),
        &group_owner,
        &payer,
        recent_blockhash,
        &mut banks_client,
        eth_pubkey,
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::validate_signature(
            &id(),
            &valid_signer.pubkey(),
            &signer_group.pubkey(),
            signature_param,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn validate_wrong_signature() {
    let mut rng = thread_rng();
    let key: [u8; 32] = rng.gen();
    let priv_key = SecretKey::parse(&key).unwrap();
    let secp_pubkey = PublicKey::from_secret_key(&priv_key);
    let eth_pubkey = construct_eth_pubkey(&secp_pubkey);

    let message = vec![5; 20];

    let mut hasher = sha3::Keccak256::new();
    hasher.update(&message);

    let message_hash = hasher.finalize();
    let mut message_hash_arr = [0u8; 32];
    message_hash_arr.copy_from_slice(&message_hash.as_slice());
    let message = Message::parse(&message_hash_arr);
    let (signature, recovery_id) = secp256k1::sign(&message, &priv_key);
    let signature_arr = signature.serialize();

    let signature_param = instruction::Signature {
        signature: signature_arr,
        recovery_id: recovery_id.serialize(),
        message: message_hash_arr,
    };

    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let valid_signer = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &valid_signer,
        state::ValidSigner::LEN,
    )
    .await
    .unwrap();

    let key_malicious: [u8; 32] = rng.gen();
    let priv_key_malicious = SecretKey::parse(&key_malicious).unwrap();
    let secp_pubkey_malicious = PublicKey::from_secret_key(&priv_key_malicious);
    let eth_pubkey_malicious = construct_eth_pubkey(&secp_pubkey_malicious);
    process_tx_init_valid_signer(
        &valid_signer.pubkey(),
        &signer_group.pubkey(),
        &group_owner,
        &payer,
        recent_blockhash,
        &mut banks_client,
        eth_pubkey_malicious,
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::validate_signature(
            &id(),
            &valid_signer.pubkey(),
            &signer_group.pubkey(),
            signature_param,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn validate_signature_with_wrong_message() {
    let mut rng = thread_rng();
    let key: [u8; 32] = rng.gen();
    let priv_key = SecretKey::parse(&key).unwrap();
    let secp_pubkey = PublicKey::from_secret_key(&priv_key);
    let eth_pubkey = construct_eth_pubkey(&secp_pubkey);

    let message = vec![5; 20];

    let mut hasher = sha3::Keccak256::new();
    hasher.update(&message);

    let message_hash = hasher.finalize();
    let mut message_hash_arr = [0u8; 32];
    message_hash_arr.copy_from_slice(&message_hash.as_slice());
    let message = Message::parse(&message_hash_arr);
    let (signature, recovery_id) = secp256k1::sign(&message, &priv_key);
    let signature_arr = signature.serialize();

    let signature_param = instruction::Signature {
        signature: signature_arr,
        recovery_id: recovery_id.serialize(),
        message: [7u8; 32],
    };

    let (mut banks_client, payer, recent_blockhash, signer_group, group_owner) = setup().await;

    process_tx_init_signer_group(
        &signer_group.pubkey(),
        &group_owner.pubkey(),
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await
    .unwrap();

    let valid_signer = Keypair::new();

    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &valid_signer,
        state::ValidSigner::LEN,
    )
    .await
    .unwrap();

    process_tx_init_valid_signer(
        &valid_signer.pubkey(),
        &signer_group.pubkey(),
        &group_owner,
        &payer,
        recent_blockhash,
        &mut banks_client,
        eth_pubkey,
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::validate_signature(
            &id(),
            &valid_signer.pubkey(),
            &signer_group.pubkey(),
            signature_param,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;

    assert!(result.is_err());
}
