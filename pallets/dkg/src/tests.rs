// This file is part of Tangle.
// Copyright (C) 2022-2024 Webb Technologies Inc.
//
// Tangle is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Tangle is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Tangle.  If not, see <http://www.gnu.org/licenses/>.
use crate::{mock::*, types::FeeInfo, Error, FeeInfo as FeeInfoStorage};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use parity_scale_codec::Encode;
use sp_core::{crypto::ByteArray, ecdsa, keccak_256, sr25519};
use sp_io::crypto::{ecdsa_generate, ecdsa_sign_prehashed, sr25519_generate, sr25519_sign};
use tangle_primitives::jobs::{DKGResult, DKGSignatureResult, DkgKeyType, JobResult};

fn mock_pub_key_ecdsa() -> ecdsa::Public {
	ecdsa_generate(tangle_crypto_primitives::ROLE_KEY_TYPE, None)
}

fn mock_pub_key_sr25519() -> sr25519::Public {
	sr25519_generate(tangle_crypto_primitives::ROLE_KEY_TYPE, None)
}

fn mock_signature_ecdsa(pub_key: ecdsa::Public, role_key: ecdsa::Public) -> Vec<u8> {
	let msg = role_key.encode();
	let hash = keccak_256(&msg);
	let signature: ecdsa::Signature =
		ecdsa_sign_prehashed(tangle_crypto_primitives::ROLE_KEY_TYPE, &pub_key, &hash).unwrap();
	signature.encode()
}

fn mock_signature_sr25519(pub_key: sr25519::Public, role_key: sr25519::Public) -> Vec<u8> {
	let msg = role_key.to_vec().encode();
	let hash = keccak_256(&msg);
	let signature: sr25519::Signature =
		sr25519_sign(tangle_crypto_primitives::ROLE_KEY_TYPE, &pub_key, &hash).unwrap();
	// sanity check
	assert!(sp_io::crypto::sr25519_verify(&signature, &hash, &pub_key));
	signature.encode()
}

#[test]
fn set_fees_works() {
	new_test_ext().execute_with(|| {
		let new_fee = FeeInfo {
			base_fee: 10,
			dkg_validator_fee: 5,
			sig_validator_fee: 5,
			refresh_validator_fee: 5,
		};

		// should fail for non update origin
		assert_noop!(DKG::set_fee(RuntimeOrigin::signed(10), new_fee.clone()), BadOrigin);

		// Dispatch a signed extrinsic.
		assert_ok!(DKG::set_fee(RuntimeOrigin::signed(1), new_fee.clone()));

		assert_eq!(FeeInfoStorage::<Runtime>::get(), new_fee);
	});
}

#[test]
fn dkg_key_verifcation_works_for_ecdsa() {
	new_test_ext().execute_with(|| {
		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Ecdsa,
			key: vec![],
			participants: vec![],
			signatures: vec![],
			threshold: 2,
		};

		// should fail for empty participants
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NoParticipantsFound
		);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Ecdsa,
			key: vec![],
			participants: vec![mock_pub_key_ecdsa().as_mut().to_vec()],
			signatures: vec![],
			threshold: 2,
		};

		// should fail for empty keys/signatures
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NoSignaturesFound
		);

		// setup key/signature
		let mut pub_key = mock_pub_key_ecdsa();
		let signature = mock_signature_ecdsa(pub_key, pub_key);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Ecdsa,
			key: vec![],
			participants: vec![mock_pub_key_ecdsa().as_mut().to_vec()],
			signatures: vec![signature.clone()],
			threshold: 1,
		};

		// should fail for less than threshold
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NotEnoughSigners
		);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Ecdsa,
			key: pub_key.0.to_vec(),
			participants: vec![pub_key.as_mut().to_vec()],
			signatures: vec![signature.clone(), signature.clone()],
			threshold: 1,
		};

		// should fail for duplicate signers
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::DuplicateSignature
		);

		// works correctly when all params as expected
		let mut participant_one = mock_pub_key_ecdsa();
		let mut participant_two = mock_pub_key_ecdsa();
		let signature_one = mock_signature_ecdsa(participant_one, participant_one);
		let signature_two = mock_signature_ecdsa(participant_two, participant_one);
		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Ecdsa,
			key: participant_one.to_raw_vec(),
			participants: vec![
				participant_one.as_mut().to_vec(),
				participant_two.as_mut().to_vec(),
			],
			signatures: vec![signature_two, signature_one],
			threshold: 1,
		};

		// should fail for signing different keys
		assert_ok!(DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),);
	});
}

#[test]
fn dkg_key_verifcation_works_for_schnorr() {
	new_test_ext().execute_with(|| {
		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Schnorr,
			key: mock_pub_key_sr25519().to_vec(),
			participants: vec![],
			signatures: vec![],
			threshold: 2,
		};

		// should fail for empty participants
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NoParticipantsFound
		);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Schnorr,
			key: vec![],
			participants: vec![mock_pub_key_sr25519().as_mut().to_vec()],
			signatures: vec![],
			threshold: 2,
		};

		// should fail for empty keys/signatures
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NoSignaturesFound
		);

		// setup key/signature
		let mut pub_key = mock_pub_key_sr25519();
		let signature = mock_signature_sr25519(pub_key, pub_key);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Schnorr,
			key: pub_key.to_vec(),
			participants: vec![mock_pub_key_sr25519().as_mut().to_vec()],
			signatures: vec![signature.clone()],
			threshold: 1,
		};

		// should fail for less than threshold
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::NotEnoughSigners
		);

		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Schnorr,
			key: pub_key.to_vec(),
			participants: vec![pub_key.as_mut().to_vec()],
			signatures: vec![signature.clone(), signature.clone()],
			threshold: 1,
		};

		// should fail for duplicate signers
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),
			Error::<Runtime>::DuplicateSignature
		);

		// works correctly when all params as expected
		let mut participant_one = mock_pub_key_sr25519();
		let mut participant_two = mock_pub_key_sr25519();
		let signature_one = mock_signature_sr25519(participant_one, participant_one);
		let signature_two = mock_signature_sr25519(participant_two, participant_one);
		let job_to_verify = DKGResult {
			key_type: DkgKeyType::Schnorr,
			key: participant_one.to_raw_vec(),
			participants: vec![
				participant_one.as_mut().to_vec(),
				participant_two.as_mut().to_vec(),
			],
			signatures: vec![signature_two, signature_one],
			threshold: 1,
		};

		// should fail for signing different keys
		assert_ok!(DKG::verify(JobResult::DKGPhaseOne(job_to_verify)),);
	});
}

#[test]
fn dkg_signature_verifcation_works_ecdsa() {
	new_test_ext().execute_with(|| {
		// setup key/signature
		let pub_key = mock_pub_key_ecdsa();
		let signature = mock_signature_ecdsa(pub_key, mock_pub_key_ecdsa());

		let job_to_verify: DKGSignatureResult = DKGSignatureResult {
			key_type: DkgKeyType::Ecdsa,
			signature,
			data: pub_key.to_raw_vec(),
			signing_key: pub_key.to_raw_vec(),
		};

		// should fail for invalid keys
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseTwo(job_to_verify)),
			Error::<Runtime>::SigningKeyMismatch
		);

		let signature = mock_signature_ecdsa(pub_key, pub_key);
		let job_to_verify: DKGSignatureResult = DKGSignatureResult {
			key_type: DkgKeyType::Ecdsa,
			signature,
			data: pub_key.to_raw_vec(),
			signing_key: pub_key.to_raw_vec(),
		};

		// should work with correct params
		assert_ok!(DKG::verify(JobResult::DKGPhaseTwo(job_to_verify)));
	});
}

#[test]
fn dkg_signature_verifcation_works_schnorr() {
	new_test_ext().execute_with(|| {
		// setup key/signature
		let pub_key = mock_pub_key_sr25519();
		let signature = mock_signature_sr25519(pub_key, mock_pub_key_sr25519());

		let job_to_verify: DKGSignatureResult = DKGSignatureResult {
			key_type: DkgKeyType::Schnorr,
			signature,
			data: pub_key.to_raw_vec(),
			signing_key: pub_key.to_raw_vec(),
		};

		// should fail for invalid keys
		assert_noop!(
			DKG::verify(JobResult::DKGPhaseTwo(job_to_verify)),
			Error::<Runtime>::InvalidSignature
		);

		let signature = mock_signature_sr25519(pub_key, pub_key);
		let job_to_verify: DKGSignatureResult = DKGSignatureResult {
			key_type: DkgKeyType::Schnorr,
			signature,
			data: pub_key.to_raw_vec(),
			signing_key: pub_key.to_raw_vec(),
		};

		// should work with correct params
		assert_ok!(DKG::verify(JobResult::DKGPhaseTwo(job_to_verify)));
	});
}
