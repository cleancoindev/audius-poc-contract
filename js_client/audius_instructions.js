require('dotenv').config();
const solanaWeb3 = require("@solana/web3.js");
const crypto = require('crypto');
const keccak256 = require('keccak256');
const secp256k1 = require("secp256k1");
const eth_utils = require("ethereumjs-util");

let SIGNER_GROUP_SIZE = 33;
let VALID_SIGNER_SIZE = 53;
let AUDIUS_PROGRAM = new solanaWeb3.PublicKey("3QqhXLvBgPZ4DCV3YjyzpiQWfeR4Lf2bSKqSnj5c8wkE");
let INSTRUCTIONS_PROGRAM = new solanaWeb3.PublicKey("Sysvar1nstructions1111111111111111111111111");

let feePayer = new solanaWeb3.Account([252,1,35,131,28,114,106,11,143,29,15,86,81,148,58,2,176,19,127,110,76,255,249,56,140,236,31,209,51,176,103,166,231,243,24,228,226,124,136,74,78,251,163,47,230,6,142,27,156,140,246,92,108,114,163,237,226,243,170,124,76,24,62,125]);
let owner = new solanaWeb3.Account([63,181,8,61,246,121,106,102,159,113,145,62,38,196,23,242,102,18,191,255,46,250,34,47,102,160,157,129,21,233,209,194,32,76,67,148,133,69,126,66,181,10,4,130,39,21,204,15,97,166,77,87,142,255,146,170,86,42,173,154,120,29,56,211 ]);

let url = solanaWeb3.clusterApiUrl('devnet', false)

let devnetConnection = new solanaWeb3.Connection(url);

async function newSystemAccountWithAirdrop(connection, lamports) {
    const account = new solanaWeb3.Account();
    await connection.requestAirdrop(account.publicKey, lamports);
    return account;
  }

function newProgramAccount(newAccount, lamports, space) {
    let instruction = solanaWeb3.SystemProgram.createAccount({
        fromPubkey: feePayer.publicKey,
        newAccountPubkey: newAccount.publicKey,
        lamports,
        space, // data space
        programId: AUDIUS_PROGRAM,
      });
    
    return instruction;
}

async function validateSignature(validSigner, privateKey, message) {
    let privKey = Buffer.from(privateKey, "hex")
    let pubKey = secp256k1.publicKeyCreate(privKey, false);
    
    let validSignerPubK = new solanaWeb3.PublicKey(validSigner);
    let accInfo = await devnetConnection.getAccountInfo(validSignerPubK);
    let signerGroup = new solanaWeb3.PublicKey(accInfo.data.toJSON().data.slice(1, 33))  // cut off version and eth address from valid signer data

    let msg = Buffer.from(message).toJSON().data;

    let msg_hash = keccak256(msg);

    const sigObj = secp256k1.ecdsaSign(Uint8Array.from(msg_hash), privKey);

    let transaction = new solanaWeb3.Transaction();
    let instruction_data = [3];
    instruction_data = instruction_data.concat(Array.from(sigObj.signature));
    instruction_data = instruction_data.concat([sigObj.recid]);
    instruction_data = instruction_data.concat(msg);

    let secpInstruction = solanaWeb3.Secp256k1Program.createInstructionWithPublicKey({publicKey: pubKey, message: msg, signature: sigObj.signature, recoveryId: sigObj.recid});

    transaction.add(secpInstruction);

    transaction.add({keys: [{pubkey: validSignerPubK, isSigner: false, isWritable: false},
                            {pubkey: signerGroup, isSigner: false, isWritable: false},
                            {pubkey: INSTRUCTIONS_PROGRAM, isSigner: false, isWritable: false},],
                    programId: AUDIUS_PROGRAM,
                    data: Buffer.from(instruction_data)});
    
    let signature = await solanaWeb3.sendAndConfirmTransaction(devnetConnection, transaction, [feePayer]);

    console.log("Signature: ", signature);
}

exports.validateSignature = validateSignature;