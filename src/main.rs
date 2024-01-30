#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(non_snake_case)]
#![allow(clippy::clone_on_copy)]
use std::{fs::File, io::Read, ops::Mul};

use ark_bls12_381::{g2::Config, Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{
  hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
  pairing::Pairing,
  CurveGroup, Group,
};
use ark_ff::field_hashers::DefaultFieldHasher;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use log::{info, warn};
use prompt::{puzzle, welcome};
use sha2::Sha256;

#[derive(Debug)]
pub enum Error {
  InvalidMsg,
}

fn hasher() -> MapToCurveBasedHasher<G2Projective, DefaultFieldHasher<Sha256, 128>, WBMap<Config>> {
  MapToCurveBasedHasher::<G2Projective, DefaultFieldHasher<Sha256, 128>, WBMap<Config>>::new(&[
    1, 3, 3, 7,
  ])
  .unwrap()
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct ElGamal(G1Affine, G1Affine);

impl ElGamal {
  pub fn hash_to_curve(&self) -> G2Affine {
    let mut data = Vec::new();
    self.serialize_uncompressed(&mut data).unwrap();

    hasher().hash(&data).unwrap()
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Message(G1Affine);

struct Sender {
  pub sk: Fr,
  pub pk: G1Affine,
}

pub struct Receiver {
  pk: G1Affine,
}

pub struct Auditor {}

impl Sender {
  pub fn send(&self, m: Message, r: &Receiver) -> ElGamal {
    let c_2: G1Affine = (r.pk.mul(&self.sk) + m.0).into_affine();
    ElGamal(self.pk, c_2)
  }

  pub fn authenticate(&self, c: &ElGamal) -> G2Affine {
    let hash_c = c.hash_to_curve();
    hash_c.mul(&self.sk).into_affine()
  }
}

impl Auditor {
  pub fn check_auth(sender_pk: G1Affine, c: &ElGamal, s: G2Affine) -> bool {
    let lhs = { Bls12_381::pairing(G1Projective::generator(), s) };

    let hash_c = c.hash_to_curve();
    let rhs = { Bls12_381::pairing(sender_pk, hash_c) };

    lhs == rhs
  }
}

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct Blob {
  pub sender_pk: G1Affine,
  pub c:         ElGamal,
  pub s:         G2Affine,
  pub rec_pk:    G1Affine,
}

fn generate_message_space() -> [Message; 10] {
  let g1 = G1Projective::generator();
  let msgs = [
    390183091831u64,
    4987238947234982,
    84327489279482,
    8492374892742,
    5894274824234,
    4982748927426,
    48248927348927427,
    489274982749828,
    99084321987189371,
    8427489729843712893,
  ];
  msgs
    .iter()
    .map(|&msg_i| Message(g1.mul(Fr::from(msg_i)).into_affine()))
    .collect::<Vec<_>>()
    .try_into()
    .unwrap()
}

pub fn main() {
  env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
  welcome();
  puzzle(PUZZLE_DESCRIPTION);

  let _messages = generate_message_space();

  let mut file = File::open("blob.bin").unwrap();
  let mut data = Vec::new();
  file.read_to_end(&mut data).unwrap();
  let blob = Blob::deserialize_uncompressed(data.as_slice()).unwrap();

  // ensure that blob is correct
  assert!(Auditor::check_auth(blob.sender_pk, &blob.c, blob.s));
  info!("auditor happy");

  // Implement your attack here, to find the index of the encrypted message
  let sender_pk = blob.sender_pk;
  let c = blob.c.clone();
  let c1 = blob.c.1.clone();
  let s = blob.s;
  let rec_pk = blob.rec_pk;

  for (i, m) in _messages.into_iter().enumerate() {
    let a = (c1 - m.0).into_affine();
    let e = check_auth_evil(a, &c, s, rec_pk);
    if e {
      warn!("found message at index {}", i);
      warn!("win");
      break;
    }
  }

  // End of attack
}

pub fn check_auth_evil(s1s2_G: G1Affine, c: &ElGamal, s: G2Affine, receiver_pk: G1Affine) -> bool {
  // let lhs = { Bls12_381::pairing(G1Projective::generator(), s) };
  // (sk_r * G1, sk_s * H(.))
  let lhs = { Bls12_381::pairing(receiver_pk, s) };
  // let lhs = { receiver_pk, s) };

  let hash_c = c.hash_to_curve();
  // (sk_r*sk_s * G1, H(.))
  let rhs = { Bls12_381::pairing(s1s2_G, hash_c) };

  lhs == rhs
}
const PUZZLE_DESCRIPTION: &str = r"
Bob designed a new one time scheme, that's based on the tried and true method of encrypt + sign. He combined ElGamal encryption with BLS signatures in a clever way, such that you use pairings to verify the encrypted message was not tampered with. Alice, then, figured out a way to reveal the plaintexts...
";
