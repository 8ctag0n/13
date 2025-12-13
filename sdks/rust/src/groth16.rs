use crate::circuits::VerificationKeyJson;
use anyhow::{anyhow, Context, Result};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_ec::bn::Bn;
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use std::str::FromStr;

pub fn vk_from_json(vk: &VerificationKeyJson) -> Result<VerifyingKey<Bn254>> {
    let alpha_g1 = parse_g1(&vk.vk_alpha_1)?;
    let beta_g2 = parse_g2(&vk.vk_beta_2)?;
    let gamma_g2 = parse_g2(&vk.vk_gamma_2)?;
    let delta_g2 = parse_g2(&vk.vk_delta_2)?;

    let ic = vk
        .ic
        .iter()
        .map(|pt| parse_g1(pt))
        .collect::<Result<Vec<_>>>()?;

    Ok(VerifyingKey {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1: ic,
    })
}

pub fn proof_from_json(proof: &serde_json::Value) -> Result<Proof<Bn254>> {
    let pi_a = parse_g1(
        proof
            .get("pi_a")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("missing pi_a"))?,
    )?;
    let pi_b = parse_g2(
        proof
            .get("pi_b")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("missing pi_b"))?,
    )?;
    let pi_c = parse_g1(
        proof
            .get("pi_c")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("missing pi_c"))?,
    )?;
    Ok(Proof { a: pi_a, b: pi_b, c: pi_c })
}

pub fn prepare_vk(vk: &VerifyingKey<Bn254>) -> PreparedVerifyingKey<Bn254> {
    prepare_verifying_key(vk)
}

pub fn verify(proof: &Proof<Bn254>, public_inputs: &[Fr], pvk: &PreparedVerifyingKey<Bn254>) -> bool {
    Groth16::<Bn254>::verify_with_processed_vk(pvk, public_inputs, proof).unwrap_or(false)
}

fn fq_from_dec(dec: &str) -> Result<Fq> {
    let f = Fq::from_str(dec).map_err(|e| anyhow!("invalid field element: {e:?}"))?;
    Ok(f)
}

fn parse_g1_strs(coords: &[String]) -> Result<G1Affine> {
    if coords.len() < 2 {
        return Err(anyhow!("invalid g1 coords"));
    }
    let fx = fq_from_dec(&coords[0])?;
    let fy = fq_from_dec(&coords[1])?;
    Ok(G1Affine::new_unchecked(fx, fy))
}

fn parse_g1(coords: &[String]) -> Result<G1Affine> {
    parse_g1_strs(coords)
}

fn parse_g2(coords: &[Vec<String>]) -> Result<G2Affine> {
    if coords.len() < 2 {
        return Err(anyhow!("invalid g2 coords"));
    }
    // coords[0] = [x0, x1], coords[1] = [y0, y1]
    let x_arr = coords[0].clone();
    let y_arr = coords[1].clone();
    if x_arr.len() < 2 || y_arr.len() < 2 {
        return Err(anyhow!("invalid g2 coords length"));
    }
    let x0 = fq_from_dec(&x_arr[0])?;
    let x1 = fq_from_dec(&x_arr[1])?;
    let y0 = fq_from_dec(&y_arr[0])?;
    let y1 = fq_from_dec(&y_arr[1])?;
    Ok(G2Affine::new_unchecked(Fq2::new(x0, x1), Fq2::new(y0, y1)))
}
