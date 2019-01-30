# Changelog

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
