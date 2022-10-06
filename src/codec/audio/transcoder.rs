//! Audio transcoder.
//!
//! This module contains just a convenience struct combining
//! audio decoder/resampler/encoder into a single pipeline.

use std::collections::VecDeque;
use std::convert::TryInto;

use crate::Error;

use crate::{
    codec::{
        audio::{
            AudioDecoder, AudioDecoderBuilder, AudioEncoder, AudioEncoderBuilder, AudioFrame,
            AudioResampler,
        },
        AudioCodecParameters, CodecError, Decoder, Encoder,
    },
    packet::Packet,
    time::TimeBase,
};

/// Builder for the AudioTranscoder.
pub struct AudioTranscoderBuilder {
    input: AudioCodecParameters,
    output: AudioCodecParameters,

    decoder_builder: AudioDecoderBuilder,
    encoder_builder: AudioEncoderBuilder,
}

impl AudioTranscoderBuilder {
    /// Create a new builder.
    fn new(input: AudioCodecParameters, output: AudioCodecParameters) -> Result<Self, Error> {
        let decoder_builder = AudioDecoder::from_codec_parameters(&input)?;
        let encoder_builder = AudioEncoder::from_codec_parameters(&output)?;

        let res = Self {
            input,
            output,

            decoder_builder,
            encoder_builder,
        };

        Ok(res)
    }

    /// Set a decoder option.
    pub fn set_decoder_option<V>(mut self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        self.decoder_builder = self.decoder_builder.set_option(name, value);
        self
    }

    /// Set an encoder option.
    pub fn set_encoder_option<V>(mut self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        self.encoder_builder = self.encoder_builder.set_option(name, value);
        self
    }

    /// Build the transcoder.
    pub fn build(self) -> Result<AudioTranscoder, Error> {
        let decoder = self
            .decoder_builder
            .time_base(TimeBase::new(
                1,
                self.input
                    .sample_rate()
                    .try_into()
                    .map_err(|e| Error::new(e))?,
            ))
            .build()?;

        let encoder = self
            .encoder_builder
            .time_base(TimeBase::new(
                1,
                self.output
                    .sample_rate()
                    .try_into()
                    .map_err(|e| Error::new(e))?,
            ))
            .build()?;

        let resampler = AudioResampler::builder()
            .source_channel_layout(self.input.channel_layout())
            .source_sample_format(self.input.sample_format())
            .source_sample_rate(self.input.sample_rate())
            .target_channel_layout(self.output.channel_layout())
            .target_sample_format(self.output.sample_format())
            .target_sample_rate(self.output.sample_rate())
            .target_frame_samples(encoder.samples_per_frame())
            .build()?;

        let res = AudioTranscoder {
            audio_decoder: decoder,
            audio_encoder: encoder,
            audio_resampler: resampler,

            ready: VecDeque::new(),
        };

        Ok(res)
    }
}

/// Audio transcoder.
///
/// # Transcoder operation
/// 1. Push a packet to the transcoder.
/// 2. Take all packets from the transcoder until you get None.
/// 3. If there are more packets to be transcoded, continue with 1.
/// 4. Flush the transcoder.
/// 5. Take all packets from the transcoder until you get None.
pub struct AudioTranscoder {
    audio_decoder: AudioDecoder,
    audio_encoder: AudioEncoder,
    audio_resampler: AudioResampler,

    ready: VecDeque<Packet>,
}

impl AudioTranscoder {
    /// Create a new transcoder for a given input and output.
    pub fn new(
        input: AudioCodecParameters,
        output: AudioCodecParameters,
    ) -> Result<AudioTranscoder, Error> {
        AudioTranscoderBuilder::new(input, output)?.build()
    }

    /// Create a new transcoder builder for a given input and output.
    pub fn builder(
        input: AudioCodecParameters,
        output: AudioCodecParameters,
    ) -> Result<AudioTranscoderBuilder, Error> {
        AudioTranscoderBuilder::new(input, output)
    }

    /// Get codec parameters of the transcoded stream.
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        self.audio_encoder.codec_parameters()
    }

    /// Push a given packet to the transcoder.
    ///
    /// # Panics
    /// The method panics if the operation is not expected (i.e. another
    /// operation needs to be done).
    pub fn push(&mut self, packet: Packet) -> Result<(), Error> {
        self.try_push(packet).map_err(|err| err.unwrap_inner())
    }

    /// Push a given packet to the transcoder.
    pub fn try_push(&mut self, packet: Packet) -> Result<(), CodecError> {
        if !self.ready.is_empty() {
            return Err(CodecError::again(
                "take all transcoded packets before pushing another packet for transcoding",
            ));
        }

        self.push_to_decoder(packet)?;

        Ok(())
    }

    /// Flush the transcoder.
    ///
    /// # Panics
    /// The method panics if the operation is not expected (i.e. another
    /// operation needs to be done).
    pub fn flush(&mut self) -> Result<(), Error> {
        self.try_flush().map_err(|err| err.unwrap_inner())
    }

    /// Flush the transcoder.
    pub fn try_flush(&mut self) -> Result<(), CodecError> {
        if !self.ready.is_empty() {
            return Err(CodecError::again(
                "take all transcoded packets before flushing the transcoder",
            ));
        }

        self.flush_decoder()?;
        self.flush_resampler()?;
        self.flush_encoder()?;

        Ok(())
    }

    /// Take the next packet from the transcoder.
    pub fn take(&mut self) -> Result<Option<Packet>, Error> {
        Ok(self.ready.pop_front())
    }

    /// Push a given packet to the internal decoder, take all decoded frames
    /// and pass them to the push_to_resampler method.
    fn push_to_decoder(&mut self, packet: Packet) -> Result<(), CodecError> {
        self.audio_decoder.try_push(packet)?;

        while let Some(frame) = self.audio_decoder.take()? {
            // XXX: this is to skip the initial padding; a correct solution
            // would be to skip a given number of samples
            if frame.pts().timestamp() >= 0 {
                self.push_to_resampler(frame)?;
            }
        }

        Ok(())
    }

    /// Push a given frame to the internal resampler, take all resampled frames
    /// and pass them to the push_to_encoder method.
    fn push_to_resampler(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        self.audio_resampler.try_push(frame)?;

        while let Some(frame) = self.audio_resampler.take()? {
            self.push_to_encoder(frame)?;
        }

        Ok(())
    }

    /// Push a given frame to the internal encoder, take all encoded packets
    /// and push them to the internal ready queue.
    fn push_to_encoder(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        self.audio_encoder.try_push(frame)?;

        while let Some(packet) = self.audio_encoder.take()? {
            self.push_to_output(packet);
        }

        Ok(())
    }

    /// Push a given packet to the output buffer.
    fn push_to_output(&mut self, packet: Packet) {
        self.ready.push_back(packet);
    }

    /// Flush the internal decoder, take all decoded frames and pass them to
    /// the push_to_resampler method.
    fn flush_decoder(&mut self) -> Result<(), CodecError> {
        self.audio_decoder.try_flush()?;

        while let Some(frame) = self.audio_decoder.take()? {
            self.push_to_resampler(frame)?;
        }

        Ok(())
    }

    /// Flush the internal resampler, take all resampled frames and pass them
    /// to the push_to_encoder method.
    fn flush_resampler(&mut self) -> Result<(), CodecError> {
        self.audio_resampler.try_flush()?;

        while let Some(frame) = self.audio_resampler.take()? {
            self.push_to_encoder(frame)?;
        }

        Ok(())
    }

    /// Flush the internal encoder, take all encoded packets and push them into
    /// the internal ready queue.
    fn flush_encoder(&mut self) -> Result<(), CodecError> {
        self.audio_encoder.try_flush()?;

        while let Some(packet) = self.audio_encoder.take()? {
            self.push_to_output(packet);
        }

        Ok(())
    }
}
