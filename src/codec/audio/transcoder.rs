use std::collections::VecDeque;

use crate::Error;

use crate::codec::audio::{AudioDecoder, AudioEncoder, AudioFrame, AudioResampler};
use crate::codec::{AudioCodecParameters, CodecError, Decoder, ErrorKind};
use crate::packet::Packet;

/// Audio transcoder.
///
/// # Transcoder operation
/// 1. Push a packet to the transcoder.
/// 2. Take all packets from the transcoder until you get None.
/// 3. If there are more packets to be transcoded, continue with 1.
/// 4. Flush the transcoder.
/// 5. Take all packets from the transcoder until you get None.
///
/// Timestamps of input packets are expected to be in microseconds. Timestamps
/// of output packets will be in microseconds as well.
pub struct AudioTranscoder {
    audio_decoder: AudioDecoder,
    audio_encoder: AudioEncoder,
    audio_resampler: AudioResampler,

    input_sample_rate: u32,
    output_sample_rate: u32,

    ready: VecDeque<Packet>,
}

impl AudioTranscoder {
    /// Create a new transcoder for a given input and output.
    pub fn new(
        input: &AudioCodecParameters,
        output: &AudioCodecParameters,
    ) -> Result<AudioTranscoder, Error> {
        let decoder = AudioDecoder::from_codec_parameters(input)?;

        let encoder = AudioEncoder::from_codec_parameters(output)?
            .time_base(1, output.sample_rate())
            .build()?;

        let resampler = AudioResampler::builder()
            .source_channel_layout(input.channel_layout())
            .source_sample_format(input.sample_format())
            .source_sample_rate(input.sample_rate())
            .target_channel_layout(output.channel_layout())
            .target_sample_format(output.sample_format())
            .target_sample_rate(output.sample_rate())
            .target_frame_samples(encoder.samples_per_frame())
            .build()?;

        let res = AudioTranscoder {
            audio_decoder: decoder,
            audio_encoder: encoder,
            audio_resampler: resampler,

            input_sample_rate: input.sample_rate(),
            output_sample_rate: output.sample_rate(),

            ready: VecDeque::new(),
        };

        Ok(res)
    }

    /// Get codec parameters of the transcoded stream.
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        self.audio_encoder.codec_parameters()
    }

    /// Push a given packet to the transcoder.
    pub fn push(&mut self, packet: &Packet) -> Result<(), CodecError> {
        if !self.ready.is_empty() {
            return Err(CodecError::new(
                ErrorKind::Again,
                "take all transcoded packets before pushing another packet for transcoding",
            ));
        }

        self.push_to_decoder(packet)?;

        Ok(())
    }

    /// Flush the transcoder.
    pub fn flush(&mut self) -> Result<(), CodecError> {
        if !self.ready.is_empty() {
            return Err(CodecError::new(
                ErrorKind::Again,
                "take all transcoded packets before flushing the transcoder",
            ));
        }

        self.flush_decoder()?;
        self.flush_resampler()?;
        self.flush_encoder()?;

        Ok(())
    }

    /// Take the next packet from the transcoder.
    pub fn take(&mut self) -> Result<Option<Packet>, CodecError> {
        Ok(self.ready.pop_front())
    }

    /// Push a given packet to the internal decoder, take all decoded frames
    /// and pass them to the push_to_resampler method.
    fn push_to_decoder(&mut self, packet: &Packet) -> Result<(), CodecError> {
        self.audio_decoder.push(packet)?;

        while let Some(frame) = self.audio_decoder.take()? {
            // convert the frame timestamp from microseconds to 1 /
            // source_sample_rate time base
            let ts = frame.pts() * self.input_sample_rate as i64 / 1_000_000;

            let frame = frame.with_pts(ts);

            // XXX: this is to skip the initial padding; a correct solution
            // would be to skip a given number of samples
            if frame.pts() >= 0 {
                self.push_to_resampler(frame)?;
            }
        }

        Ok(())
    }

    /// Push a given frame to the internal resampler, take all resampled frames
    /// and pass them to the push_to_encoder method.
    fn push_to_resampler(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        self.audio_resampler.push(&frame)?;

        while let Some(frame) = self.audio_resampler.take()? {
            self.push_to_encoder(frame)?;
        }

        Ok(())
    }

    /// Push a given frame to the internal encoder, take all encoded packets
    /// and push them to the internal ready queue.
    fn push_to_encoder(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        self.audio_encoder.push(&frame)?;

        while let Some(packet) = self.audio_encoder.take()? {
            self.push_to_output(packet);
        }

        Ok(())
    }

    /// Push a given packet to the output buffer.
    fn push_to_output(&mut self, packet: Packet) {
        // convert the packet timestamp from 1 / output_sample_rate to
        // microseconds
        let ts = packet.pts() * 1_000_000 / self.output_sample_rate as i64;

        let packet = packet.with_pts(ts).with_dts(ts);

        self.ready.push_back(packet);
    }

    /// Flush the internal decoder, take all decoded frames and pass them to
    /// the push_to_resampler method.
    fn flush_decoder(&mut self) -> Result<(), CodecError> {
        self.audio_decoder.flush()?;

        while let Some(frame) = self.audio_decoder.take()? {
            self.push_to_resampler(frame)?;
        }

        Ok(())
    }

    /// Flush the internal resampler, take all resampled frames and pass them
    /// to the push_to_encoder method.
    fn flush_resampler(&mut self) -> Result<(), CodecError> {
        self.audio_resampler.flush()?;

        while let Some(frame) = self.audio_resampler.take()? {
            self.push_to_encoder(frame)?;
        }

        Ok(())
    }

    /// Flush the internal encoder, take all encoded packets and push them into
    /// the internal ready queue.
    fn flush_encoder(&mut self) -> Result<(), CodecError> {
        self.audio_encoder.flush()?;

        while let Some(packet) = self.audio_encoder.take()? {
            self.push_to_output(packet);
        }

        Ok(())
    }
}
