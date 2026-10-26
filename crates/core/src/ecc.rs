//! Shortened systematic RS(9,7), GF(256)/0x11d, roots alpha^0 and alpha^1.
use crate::Error;

pub fn mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0;
    while b != 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        let high = a & 0x80;
        a <<= 1;
        if high != 0 {
            a ^= 0x1d;
        }
        b >>= 1;
    }
    result
}

fn pow(a: u8, n: usize) -> u8 {
    (0..n).fold(1, |v, _| mul(v, a))
}
fn div(a: u8, b: u8) -> u8 {
    debug_assert_ne!(b, 0);
    mul(a, pow(b, 254))
}

fn syndromes(word: &[u8; 9]) -> (u8, u8) {
    (
        word.iter().fold(0, |v, b| v ^ b),
        word.iter().fold(0, |v, b| mul(v, 2) ^ b),
    )
}

pub fn encode(message: [u8; 7]) -> [u8; 9] {
    let mut remainder = [0; 9];
    remainder[..7].copy_from_slice(&message);
    // g(x) = x^2 + 3x + 2, highest degree coefficient first.
    for i in 0..7 {
        let leading = remainder[i];
        remainder[i + 1] ^= mul(leading, 3);
        remainder[i + 2] ^= mul(leading, 2);
    }
    remainder[..7].copy_from_slice(&message);
    remainder
}

/// Correct one unknown byte error OR up to two known byte erasures.
/// More corruption can alias another codeword: the caller MUST also check CRC.
pub fn correct(mut word: [u8; 9], erasures: &[usize]) -> Result<([u8; 9], u8), Error> {
    if erasures.len() > 2
        || erasures.iter().any(|&i| i >= 9)
        || (erasures.len() == 2 && erasures[0] == erasures[1])
    {
        return Err(Error::Ecc);
    }
    for &i in erasures {
        word[i] = 0;
    }
    let (s0, s1) = syndromes(&word);
    let repaired = match erasures {
        [] => {
            if s0 == 0 && s1 == 0 {
                return Ok((word, 0));
            }
            if s0 == 0 {
                return Err(Error::Ecc);
            }
            let location = div(s1, s0);
            let index = (0..9)
                .find(|&i| pow(2, 8 - i) == location)
                .ok_or(Error::Ecc)?;
            word[index] ^= s0;
            1
        }
        &[i] => {
            if mul(s0, pow(2, 8 - i)) != s1 {
                return Err(Error::Ecc);
            }
            word[i] ^= s0;
            1
        }
        &[i, j] => {
            let a = pow(2, 8 - i);
            let b = pow(2, 8 - j);
            let ei = div(s1 ^ mul(b, s0), a ^ b);
            word[i] ^= ei;
            word[j] ^= s0 ^ ei;
            2
        }
        _ => return Err(Error::Ecc),
    };
    if syndromes(&word) != (0, 0) {
        return Err(Error::Ecc);
    }
    Ok((word, repaired))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_single_symbol_error_and_erasure_pair() {
        let word = encode([0x10, 0x80, 0, 0, 0, 0xd3, 0x76]);
        assert_eq!(syndromes(&word), (0, 0));
        for i in 0..9 {
            for error in 1..=255 {
                let mut damaged = word;
                damaged[i] ^= error;
                assert_eq!(correct(damaged, &[]).unwrap(), (word, 1));
            }
            for j in i + 1..9 {
                let mut damaged = word;
                damaged[i] = 0x42;
                damaged[j] = 0xa5;
                assert_eq!(correct(damaged, &[i, j]).unwrap().0, word);
            }
        }
    }
}
