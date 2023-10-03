use crate::math::softmax;
use crabml::error::Error;
use crabml::error::ErrorKind;
use crabml::error::Result;
use rand::Rng;

pub struct Llama2Sampler {
    prob_index: Vec<(f32, usize)>,
    temperature: f32,
    topp: f32,
}

impl Llama2Sampler {
    pub fn new(vocab_size: usize, temperature: f32, topp: f32) -> Self {
        Self {
            prob_index: vec![(0.0, 0); vocab_size],
            temperature,
            topp,
        }
    }

    pub fn sample(&mut self, logits: &mut [f32]) -> Result<usize> {
        if self.temperature == 0.0 {
            return Self::sample_argmax(logits);
        }

        // apply the temperature to the logits. the lower the temperature,
        // the more deterministic the sampling.
        for logit in logits.iter_mut() {
            *logit /= self.temperature;
        }
        // apply softmax to the logits to get the probabilities for next token
        softmax(logits);

        // flip a (float) coin (this is our source of entropy for sampling)
        let mut rng = rand::thread_rng();
        let coin: f32 = rng.gen_range(0.0..1.0);

        // we sample from this distribution to get the next token
        if self.topp <= 0_f32 || self.topp >= 1.0_f32 {
            // simply sample from the predicted probability distribution
            Self::sample_multi(logits, coin);
        }

        Self::sample_topp(logits, self.topp, &mut self.prob_index, coin)
    }

    pub fn sample_multi(probs: &[f32], coin: f32) -> usize {
        // sample index from probabilities (they must sum to 1!)
        // coin is a random number in [0, 1), usually from random_f32()
        let mut cdf = 0_f32;
        for (i, p) in probs.iter().enumerate() {
            cdf += p;
            if cdf > coin {
                return i;
            }
        }
        probs.len() - 1 // in case of rounding errors
    }

    pub fn sample_topp(
        probs: &[f32],
        topp: f32,
        prob_index: &mut [(f32, usize)],
        coin: f32,
    ) -> Result<usize> {
        // top-p sampling (or "nucleus sampling") samples from the smallest set of
        // tokens that exceed probability topp. This way we never sample tokens that
        // have very low probabilities and are less likely to go "off the rails".
        // coin is a random number in [0, 1), usually from random_f32()

        let cutoff = (1.0_f32 - topp) / (probs.len() - 1) as f32;
        let mut n0 = 0;
        for (i, prob) in probs.iter().enumerate() {
            if *prob >= cutoff {
                prob_index[n0] = (probs[i], i);
                n0 += 1;
            }
        }
        prob_index[..n0].sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // truncate the list where cumulative probability exceeds topp
        let mut cumulative_prob = 0_f32;
        let mut last_idx = n0 - 1; // in case of rounding errors consider all elements
        for (i, prob) in prob_index[0..n0].iter().enumerate() {
            cumulative_prob += prob.0;
            if cumulative_prob > topp {
                last_idx = i;
                break; // we've exceeded topp by including last_idx
            }
        }

        // sample from the truncated list
        let r = coin * cumulative_prob;
        let mut cdf = 0_f32;
        for prob in prob_index[0..=last_idx].iter() {
            cdf += prob.0;
            if cdf > r {
                return Ok(prob.1);
            }
        }
        Ok(prob_index[last_idx].1) // in case of rounding errors
    }

    pub fn sample_argmax(probs: &[f32]) -> Result<usize> {
        probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .ok_or_else(|| Error {
                kind: ErrorKind::Unexpected,
                message: format!("failed to sample from logits"),
                cause: None,
            })
    }
}
