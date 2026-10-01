//! Build channel of this binary.
//!
//! The default log level is a release decision, not a Rust build-profile
//! decision: an optimized pre-release still has to default to `debug`, so
//! keying the default on `debug_assertions` cannot express the requirement.
//!
//! The channel is baked in at build time by `desktop/build/set-channel.mjs`
//! through the `XARCHIVE_RELEASE_CHANNEL` environment variable. `windows-release.yml`
//! derives it from the already validated release tag before the build starts.
//!
//! An unknown channel is rejected rather than silently treated as a release,
//! because silently defaulting to `info` in a pre-release would hide exactly
//! the diagnostics the pre-release exists to produce.

use std::fmt;
use std::str::FromStr;

use crate::config::LogLevel;

/// The channel a binary was built for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseChannel {
    /// Local development builds.
    Dev,
    /// Optimized pre-release builds such as `v0.2.1-pre1`.
    PreRelease,
    /// Optimized stable release builds such as `v0.2.0`.
    Release,
}

impl ReleaseChannel {
    /// The level a fresh installation defaults to for this channel.
    ///
    /// A pre-release defaults to `debug` so every module reports detailed
    /// diagnostics; a stable release defaults to `info` so detail is filtered
    /// while normal progress, warnings, and errors remain.
    pub const fn default_log_level(self) -> LogLevel {
        match self {
            Self::Dev | Self::PreRelease => LogLevel::Debug,
            Self::Release => LogLevel::Info,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::PreRelease => "prerelease",
            Self::Release => "release",
        }
    }
}

/// Returned when a build requests a channel this binary does not know.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownChannel(pub String);

impl fmt::Display for UnknownChannel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unknown release channel {:?}; expected dev, prerelease, or release",
            self.0
        )
    }
}

impl std::error::Error for UnknownChannel {}

impl FromStr for ReleaseChannel {
    type Err = UnknownChannel;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Dev),
            "prerelease" | "pre-release" | "pre" => Ok(Self::PreRelease),
            "release" | "stable" => Ok(Self::Release),
            _ => Err(UnknownChannel(value.to_owned())),
        }
    }
}

/// The channel compiled into this binary.
///
/// An unset variable means a local development build; CI always sets it
/// explicitly. A variable that is set but unparseable is a build mistake, and
/// this panics instead of guessing: defaulting it to `release` would hide the
/// pre-release diagnostics that the channel exists to surface, and defaulting
/// it to `dev` would ship a verbose binary under a stable release name. The
/// build script rejects the same value before compilation, so this is a
/// defense-in-depth failure rather than a normal path.
///
/// This is a normal function rather than a `const fn` because matching on the
/// `str` returned by `option_env!` is not allowed in a constant context.
pub fn release_channel() -> ReleaseChannel {
    match option_env!("XARCHIVE_RELEASE_CHANNEL") {
        None => ReleaseChannel::Dev,
        Some(value) => ReleaseChannel::from_str(value).unwrap_or_else(|error| panic!("{error}")),
    }
}

/// The default log level for the channel this binary was built for.
pub fn channel_default_log_level() -> LogLevel {
    release_channel().default_log_level()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_documented_channel() {
        assert_eq!(
            ReleaseChannel::from_str("dev").unwrap(),
            ReleaseChannel::Dev
        );
        assert_eq!(
            ReleaseChannel::from_str("pre-release").unwrap(),
            ReleaseChannel::PreRelease
        );
        assert_eq!(
            ReleaseChannel::from_str("PRERELEASE").unwrap(),
            ReleaseChannel::PreRelease
        );
        assert_eq!(
            ReleaseChannel::from_str("release").unwrap(),
            ReleaseChannel::Release
        );
    }

    #[test]
    fn rejects_an_unknown_channel_instead_of_defaulting() {
        // Defaulting an unrecognized value to `release` would quietly hide
        // pre-release diagnostics, so the parse has to fail.
        let error = ReleaseChannel::from_str("nightly").unwrap_err();
        assert_eq!(error.0, "nightly");
        assert!(ReleaseChannel::from_str("").is_err());
    }

    #[test]
    fn prerelease_defaults_to_debug_and_release_to_info() {
        assert_eq!(
            ReleaseChannel::PreRelease.default_log_level(),
            LogLevel::Debug
        );
        assert_eq!(ReleaseChannel::Release.default_log_level(), LogLevel::Info);
        assert_eq!(ReleaseChannel::Dev.default_log_level(), LogLevel::Debug);
    }

    #[test]
    fn the_compiled_channel_agrees_with_the_environment() {
        // A build that was given an unknown channel must fail loudly rather
        // than quietly reporting `dev`, so this test is expected to fail when
        // `XARCHIVE_RELEASE_CHANNEL` is set to something unparseable.
        match option_env!("XARCHIVE_RELEASE_CHANNEL") {
            None => assert_eq!(release_channel(), ReleaseChannel::Dev),
            Some(value) => assert_eq!(
                release_channel(),
                ReleaseChannel::from_str(value).expect("build channel must parse")
            ),
        }
    }

    #[test]
    fn the_default_level_tracks_the_compiled_channel() {
        assert_eq!(
            channel_default_log_level(),
            release_channel().default_log_level()
        );
    }
}
