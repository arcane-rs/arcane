`arcana` changelog
===================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.1.0] · 2021-06-25
[0.1.0]: /../../tree/v0.1.0

### Initially implemented

- Events
  - Traits
    - `Event`
    - `VersionedEvent`
    - `EventSourced`
    - `EventInitialised`
  - Structs
    - `EventVersion`
    - `event::Initial` specialization wrapper
  - Proc macros
    - `Event` derive
    - `VersionedEvent` derive
  - Transforming Events
    - Traits
      - `Adapter`
      - `Transformer`
      - `Adapt`
      - `Strategy`
    - Structs
      - `strategy::AsIs`
      - `strategy::Custom`
      - `strategy::Into`
      - `strategy::Skip`
      - `strategy::Split`

    

[Semantic Versioning 2.0.0]: https://semver.org
