# Changelog

## v0.15.3 (2020-09-29)

* Allow accessing audio frame samples

## v0.15.2 (2020-09-11)

* Reset codec tag when adding stream to a muxer (fixes incompatibility between
  different containers)

## v0.15.1 (2020-08-11)

* Use pkg-config for finding native dependencies

## v0.15.0 (2020-08-11)

* Polish the public API a bit
* Improve error handling
* Update README
* Add module-level docs
* Improve decoder/encoder interface
* Add better examples
* Add license file

## v0.14.0 (2020-07-02)

* Update the bytes crate

## v0.13.0 (2020-06-08)

* Allow setting encoder and decoder options

## v0.12.1 (2020-06-05)

* Avoid excessive amount of allocations in the MemWriter

## v0.12.0 (2020-01-31)

* Add interface for video frame planes

## v0.11.2 (2019-08-08)

* Add missing `impl Send` and `impl Sync` for the bitstream filter

## v0.11.1 (2019-08-06)

* Add bitstream filters

## v0.11.0 (2019-07-26)

* Add demuxer
* Refactor of the format::io module
* Improve error handling

## v0.10.2 (2019-02-20)

* Add methods for getting decoder/encoder name from codec parameters

## v0.10.1 (2019-01-31)

* Add a method for creating black video frames
* Fix output packet DTS in the audio transcoder

## v0.10.0 (2019-01-30)

* Update the API for dealing with channel layouts, sample formats and pixel formats

## v0.9.0 (2019-01-30)

* Unification of the decoder interface
* Remove the Muxer::get\_option method and allow setting options in MuxerBuilder
* Fix timestamp handling in audio resampler and audio transcoder and add gap/overlap compensation to the resampler

## v0.8.0 (2019-01-23)

* Add audio resampler
* Refactoring of the codec parameters model

## v0.7.1 (2019-01-22)

* Add audio resampler
* Allow to take codec parameters from encoders and make frame size available for audio encoders

## v0.7.0 (2019-01-21)

* Public API refinements
* New audio encoder and decoder

## v0.6.1 (2019-01-18)

* Create decoder from codec parameters
* Determine media type from codec parameters

## v0.6.0 (2018-12-19)

* Make codec parameters independent of media type

## v0.5.1 (2018-12-12)

* Implement Clone for CodecParameters

## v0.5.0 (2018-12-07)

* Redesign of the packet interface for better ergonomics

## v0.4.1 (2018-12-05)

* Fix memory leak in muxer

## v0.4.0 (2018-12-05)

* Add support for muxer runtime options

## v0.3.0 (2018-12-04)

* Add codec parameters, decoder builder and extradata

## v0.2.1 (2018-12-03)

* Allow to take the muxer output

## v0.2.0 (2018-12-03)

* Add muxer
