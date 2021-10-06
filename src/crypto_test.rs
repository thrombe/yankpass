
// code that i used to figure out what i needed to do. not unit tests or whatever

use rpassword;
use openssl;

pub fn try1() {
    // prompt for password
    let pass = rpassword::prompt_password_stdout("lol: ").unwrap();
    dbg!(&pass);


    use openssl::{rsa::{Rsa, Padding}, symm::Cipher};

    // generate keys
    let rsa = Rsa::generate(2048).unwrap();
    let private_key = rsa.private_key_to_pem_passphrase(Cipher::aes_128_cbc(), pass.as_bytes()).unwrap();
    let public_key = rsa.public_key_to_pem().unwrap();
    dbg!(&private_key, &public_key);

    // encrypt with public key
    let rsa2 = Rsa::public_key_from_pem(&public_key).unwrap();
    let mut buf = vec![0; rsa2.size() as usize];
    rsa2.public_encrypt(pass.as_bytes(), &mut buf, Padding::PKCS1).unwrap();
    dbg!(&buf);

    // decrypt with private key
    let mut buf2 = vec![0; rsa.size() as usize];
    rsa.private_decrypt(&buf, &mut buf2, Padding::PKCS1).unwrap();
    dbg!(&buf2);
}

pub fn try2() {

    use openssl::{
        rsa::{Rsa, Padding},
        encrypt::{Encrypter, Decrypter},
        pkey::PKey,
    };


    let pass = "pyassworde";

    // refrence openssl::encrypt
    // generate keys
    let keypair = Rsa::generate(2048).unwrap();
    let keypair = PKey::from_rsa(keypair).unwrap();
    let public_pem = keypair.public_key_to_pem().unwrap();
    dbg!(String::from_utf8(public_pem.clone()));

    // encrypt into a printable hex string
    let public = PKey::public_key_from_pem(&public_pem).unwrap();
    let mut encrypter = Encrypter::new(&public).unwrap();
    encrypter.set_rsa_padding(Padding::PKCS1).unwrap();
    let mut encrypted = {
        let buffer_len = encrypter.encrypt_len(&pass.as_bytes()).unwrap();
        vec![0; buffer_len]
    };
    let encrypted_len = encrypter.encrypt(&pass.as_bytes(), &mut encrypted).unwrap();
    encrypted.truncate(encrypted_len);
    let encrypted_str = {
        let mut string = String::new();
        for ele in &encrypted {
            let mut strr = format!("{:x?}", ele);
            if strr.len() != 2 {strr = "0".to_owned()+&strr}
            string += &strr;
        }
        string
    };
    dbg!(&encrypted_str);

    // dbg!(&encrypted);
    // decrypt from the encrypted hex string
    let encrypted2: Vec<u8> = {
        (0..encrypted_str.len()).step_by(2)
            .map(|i| u8::from_str_radix(
                &encrypted_str[i..i+2], 16).unwrap()
            )
            .collect()
    };
    // dbg!(&encrypted2);
    let mut decrypter = Decrypter::new(&keypair).unwrap();
    decrypter.set_rsa_padding(Padding::PKCS1).unwrap();
    let mut decrypted = {
        let buffer_len = decrypter.decrypt_len(&encrypted).unwrap();
        vec![0; buffer_len]
    };
    let decrypted_len = decrypter.decrypt(&encrypted2, &mut decrypted).unwrap();
    decrypted.truncate(decrypted_len);
    dbg!(String::from_utf8(decrypted.clone()));
}
