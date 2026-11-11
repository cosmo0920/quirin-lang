// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Declared IPv4 and IPv6 addresses with ports for `source.socket.v1`
//! (card `SOCKET-INET-ADDRESSES`).
//!
//! # What an address is here, stated before anything else
//!
//! An address is a **check-time constant**: a literal IPv4 dotted quad or a
//! literal IPv6 address, together with a port that is an integer literal or a
//! name this entry bound to a check-time-constant integer. It is parsed into
//! **bytes** here, once, and the emitted program carries those bytes rather than
//! the text — so nothing downstream re-parses an address and no two parsers can
//! disagree about which address a spelling meant. That is the same discipline
//! this route already applies to a payload byte.
//!
//! # There is no name resolution, by construction
//!
//! A hostname is not an address. `parse_socket_address` refuses one by name
//! ([`AddressRefusal::Hostname`]) rather than deferring it to a resolver, and
//! there is no resolver: DNS is the named follow-on `SOCKET-DNS`. This is not an
//! omission — a resolved name is a value the checker cannot compute, and this
//! route's whole evidence story is that everything it puts on the wire is
//! decided before emission.
//!
//! # The admitted IPv6 textual subset, decided rather than assumed
//!
//! [`parse_ipv6`] admits the RFC 4291 §2.2 forms **1** and **2**: eight
//! colon-separated groups of one to four hexadecimal digits, with at most one
//! `::` standing for one or more all-zero groups. Case is immaterial, because
//! comparison is on the parsed bytes and never on the text, so `::1` and
//! `0:0:0:0:0:0:0:1` are the same address here exactly as they are in RFC 4291.
//!
//! Two forms are refused **by name** rather than admitted:
//!
//! * **Form 3, the embedded-IPv4 spelling** (`::ffff:127.0.0.1`). It is refused
//!   because it is precisely the dual-stack shape this card's `IPV6_V6ONLY`
//!   decision declines to model: an address in that form denotes a v4 peer
//!   reached through a v6 socket, and every v6 socket an admitted program creates
//!   sets `IPV6_V6ONLY` to 1, so such an address could never be reached. Refusing
//!   it is the honest answer; admitting it would spell a capability the emitted
//!   program does not have. Named follow-on `SOCKET-INET-V6-EMBEDDED-V4`.
//! * **A zone identifier** (`fe80::1%eth0`). A zone is an interface of the
//!   running host, not a constant this checker can evaluate — the same reason a
//!   hostname is refused. Named follow-on `SOCKET-INET-V6-ZONE`.
//!
//! # An IPv4 component carries no leading zero
//!
//! `010` is refused rather than read as ten or as eight. glibc's `inet_pton`
//! refuses it, other platforms have historically read it as octal, and this route
//! runs on three platforms and pins none — so the one spelling every platform
//! agrees about is the only one admitted. The refusal is by name
//! ([`AddressRefusal::Ipv4Component`]), never a silent choice between two
//! readings.

use std::fmt;

/// The largest port number an address may name.
pub const MAX_PORT: u32 = 65535;

/// Which address family an address belongs to.
///
/// The two spaces are **disjoint** in this model: `0.0.0.0` and `::` are
/// different addresses, a socket is created in exactly one family, and a v6
/// socket of an emitted program sets `IPV6_V6ONLY` so that the C agrees with
/// that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    /// `AF_INET`.
    V4,
    /// `AF_INET6`.
    V6,
}

impl Family {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::V4 => "ipv4",
            Self::V6 => "ipv6",
        }
    }

    /// Whether this family is the one the emitted program creates with
    /// `AF_INET6`. Published as a function rather than spelled at each use so a
    /// reader of the backend sees one decision.
    #[must_use]
    pub fn is_v6(self) -> bool {
        self == Self::V6
    }
}

/// A literal address, parsed into its wire bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpAddress {
    V4([u8; 4]),
    V6([u8; 16]),
}

impl IpAddress {
    #[must_use]
    pub fn family(self) -> Family {
        match self {
            Self::V4(_) => Family::V4,
            Self::V6(_) => Family::V6,
        }
    }

    /// The address' wire bytes, network order, exactly as the emitted program
    /// carries them.
    #[must_use]
    pub fn octets(&self) -> &[u8] {
        match self {
            Self::V4(octets) => octets.as_slice(),
            Self::V6(octets) => octets.as_slice(),
        }
    }

    /// Whether this address is the family's loopback — `127.0.0.0/8` or `::1`.
    #[must_use]
    pub fn is_loopback(self) -> bool {
        match self {
            Self::V4(octets) => octets[0] == 127,
            Self::V6(octets) => octets[..15].iter().all(|byte| *byte == 0) && octets[15] == 1,
        }
    }

    /// Whether this address is the family's unspecified/any address —
    /// `0.0.0.0` or `::`. This is the one an emitted program binds as
    /// `INADDR_ANY` / `in6addr_any`, and it is spelled rather than inferred.
    #[must_use]
    pub fn is_unspecified(self) -> bool {
        self.octets().iter().all(|byte| *byte == 0)
    }
}

impl fmt::Display for IpAddress {
    /// The canonical rendering: dotted decimal for v4, and **eight uncompressed
    /// lowercase groups** for v6.
    ///
    /// Uncompressed on purpose. This text appears only in diagnostics, and a
    /// reader comparing two refusals should not have to decide whether `::1` and
    /// `0:0:0:0:0:0:0:1` are the same address — the rendering answers it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V4(octets) => {
                write!(f, "{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
            }
            Self::V6(octets) => {
                let groups: Vec<String> = (0..8)
                    .map(|group| {
                        format!(
                            "{:x}",
                            (u16::from(octets[group * 2]) << 8) | u16::from(octets[group * 2 + 1])
                        )
                    })
                    .collect();
                write!(f, "{}", groups.join(":"))
            }
        }
    }
}

/// What a port position spells.
///
/// Two forms, and deliberately the same two an operand takes: a literal, or a
/// name this entry bound to a check-time-constant integer. A computed port is
/// not a check-time constant, and a port this checker cannot evaluate is an
/// address it cannot compare against the program's own binds — which is what the
/// whole resolution below rests on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortSpec {
    /// An integer literal written at the site. Held as `u32` so a value above
    /// [`MAX_PORT`] is **refused** by the checker rather than wrapped here.
    Literal(u32),
    /// A name this entry bound with `let NAME = INTEGER;`.
    Scalar(String),
}

/// A declared address, before its port is resolved.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketAddress {
    pub ip: IpAddress,
    pub port: PortSpec,
}

impl SocketAddress {
    #[must_use]
    pub fn family(&self) -> Family {
        self.ip.family()
    }
}

impl fmt::Display for SocketAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.ip, &self.port) {
            (IpAddress::V4(_), PortSpec::Literal(port)) => write!(f, "{}:{port}", self.ip),
            (IpAddress::V4(_), PortSpec::Scalar(name)) => write!(f, "{}:{name}", self.ip),
            (IpAddress::V6(_), PortSpec::Literal(port)) => write!(f, "[{}]:{port}", self.ip),
            (IpAddress::V6(_), PortSpec::Scalar(name)) => write!(f, "[{}]:{name}", self.ip),
        }
    }
}

/// A declared address whose port the checker has evaluated.
///
/// **Port 0 is retained rather than rejected**: it is the explicit spelling of
/// "this address, a kernel-chosen port", which is the collision-free form and the
/// one every pre-existing program already gets from a bare `bind NAME;`. What
/// port 0 cannot do is *name* a socket: an ephemeral port is not known at check
/// time, so an address carrying one matches no bind and a `connect` naming it is
/// refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedAddress {
    pub ip: IpAddress,
    pub port: u16,
}

impl ResolvedAddress {
    #[must_use]
    pub fn family(self) -> Family {
        self.ip.family()
    }

    /// Whether this address names a port the kernel chooses.
    #[must_use]
    pub fn ephemeral(self) -> bool {
        self.port == 0
    }
}

impl fmt::Display for ResolvedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ip {
            IpAddress::V4(_) => write!(f, "{}:{}", self.ip, self.port),
            IpAddress::V6(_) => write!(f, "[{}]:{}", self.ip, self.port),
        }
    }
}

/// What a `connect` or a `sendto` names: a socket of this entry, or an address.
///
/// The two are not two syntaxes for one thing. A **name** is a socket this
/// program created, so the peer is known by construction. An **address** is
/// resolved against the addresses this program's own binds declared — and if
/// none matches, the refusal is the external-peer refusal, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Named(String),
    Address(SocketAddress),
}

impl Endpoint {
    /// The socket name this endpoint spells, if it spells one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name.as_str()),
            Self::Address(_) => None,
        }
    }

    #[must_use]
    pub fn address(&self) -> Option<&SocketAddress> {
        match self {
            Self::Named(_) => None,
            Self::Address(address) => Some(address),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => f.write_str(name),
            Self::Address(address) => write!(f, "{address}"),
        }
    }
}

/// Why a spelling is not an address this route admits.
///
/// Every arm is a refusal **by name**. None of them is a fallback, a guess or a
/// deferral to something that runs later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddressRefusal {
    /// Nothing at all at an address position.
    Empty,
    /// No `:` and therefore no port. An address of this route always carries
    /// one: a socket without a port is not an endpoint.
    NoPort,
    /// The host is a name rather than a literal address.
    Hostname(String),
    /// An IPv6 address spelled without its brackets, where the `:` separating
    /// host from port cannot be told from the `:` inside the host.
    BracketsRequired(String),
    /// Brackets around something that is not an IPv6 address.
    BracketedHost(String),
    /// An IPv4 component that is not one.
    Ipv4Component(String),
    /// An IPv6 host this route's admitted textual subset does not cover.
    Ipv6Shape(String),
    /// The embedded-IPv4 form (`::ffff:1.2.3.4`).
    EmbeddedIpv4(String),
    /// A zone identifier (`fe80::1%eth0`).
    ZoneIdentifier(String),
    /// A port position that is neither an integer literal nor a name.
    PortShape(String),
    /// An integer literal port outside `0..=65535`.
    PortRange(String),
}

impl fmt::Display for AddressRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str(
                "this position takes an address; `source.socket.v1` spells one \
                 `A.B.C.D:PORT` or `[IPV6]:PORT`",
            ),
            Self::NoPort => f.write_str(
                "this address carries no port; `source.socket.v1` spells an address \
                 `A.B.C.D:PORT` or `[IPV6]:PORT`, because a socket without a port is not \
                 an endpoint",
            ),
            Self::Hostname(text) => write!(
                f,
                "`{text}` is a name, not an address; `source.socket.v1` performs no name \
                 resolution, because a resolved name is a value this checker cannot compute \
                 and every address it admits is decided before emission (`SOCKET-DNS`)"
            ),
            Self::BracketsRequired(text) => write!(
                f,
                "`{text}` looks like an IPv6 address written without brackets, where the `:` \
                 before the port cannot be told from the `:` inside the address; \
                 `source.socket.v1` spells an IPv6 endpoint `[{text}]:PORT`"
            ),
            Self::BracketedHost(text) => write!(
                f,
                "`[{text}]` is not an IPv6 address; brackets spell an IPv6 host and nothing \
                 else, and an IPv4 endpoint is written `A.B.C.D:PORT` without them"
            ),
            Self::Ipv4Component(text) => write!(
                f,
                "`{text}` is not an IPv4 address: each of its four components is one to three \
                 decimal digits in 0..=255 with no leading zero. A leading zero is refused \
                 rather than read, because platforms disagree about whether it is decimal or \
                 octal and this route pins none of them"
            ),
            Self::Ipv6Shape(text) => write!(
                f,
                "`{text}` is not an IPv6 address `source.socket.v1` admits: eight groups of \
                 one to four hexadecimal digits separated by `:`, with at most one `::` \
                 standing for one or more all-zero groups"
            ),
            Self::EmbeddedIpv4(text) => write!(
                f,
                "`{text}` is the embedded-IPv4 IPv6 form, which `source.socket.v1` refuses by \
                 name: it denotes an IPv4 peer reached through an IPv6 socket, and every IPv6 \
                 socket this route creates sets `IPV6_V6ONLY`, so no admitted program could \
                 reach it (`SOCKET-INET-V6-EMBEDDED-V4`)"
            ),
            Self::ZoneIdentifier(text) => write!(
                f,
                "`{text}` carries a zone identifier, which names an interface of the running \
                 host rather than a constant this checker can evaluate \
                 (`SOCKET-INET-V6-ZONE`)"
            ),
            Self::PortShape(text) => write!(
                f,
                "`{text}` is not a port; `source.socket.v1` takes an integer literal or a name \
                 this entry bound to a check-time-constant integer, so that every address it \
                 compares is decided before emission"
            ),
            Self::PortRange(text) => write!(
                f,
                "`{text}` is not a port `source.socket.v1` models; a port is 0 through \
                 {MAX_PORT}, and 0 is the explicit spelling of a kernel-chosen port"
            ),
        }
    }
}

/// Parse one address spelling into its bytes and its port.
///
/// # Errors
///
/// [`AddressRefusal`] naming exactly which part of the spelling is not an
/// address.
pub fn parse_socket_address(text: &str) -> Result<SocketAddress, AddressRefusal> {
    if text.is_empty() {
        return Err(AddressRefusal::Empty);
    }
    let (host, port) = split_host_and_port(text)?;
    let ip = if let Some(inner) = host.strip_prefix('[') {
        let Some(inner) = inner.strip_suffix(']') else {
            return Err(AddressRefusal::Ipv6Shape(host.to_owned()));
        };
        IpAddress::V6(parse_ipv6(inner)?)
    } else if host.contains(':') {
        return Err(AddressRefusal::BracketsRequired(host.to_owned()));
    } else {
        IpAddress::V4(parse_ipv4(host)?)
    };
    Ok(SocketAddress {
        ip,
        port: parse_port(port)?,
    })
}

/// Split `HOST:PORT` at the separator, honouring the bracketed IPv6 form.
///
/// The brackets are what make this total: inside them every `:` belongs to the
/// host, and the separator is the one immediately after `]`. Outside them a host
/// containing a `:` is refused rather than split at a guess.
fn split_host_and_port(text: &str) -> Result<(&str, &str), AddressRefusal> {
    if text.starts_with('[') {
        let Some(end) = text.find(']') else {
            return Err(AddressRefusal::Ipv6Shape(text.to_owned()));
        };
        let rest = &text[end + 1..];
        let Some(port) = rest.strip_prefix(':') else {
            return Err(AddressRefusal::NoPort);
        };
        return Ok((&text[..=end], port));
    }
    let Some(separator) = text.rfind(':') else {
        return Err(AddressRefusal::NoPort);
    };
    Ok((&text[..separator], &text[separator + 1..]))
}

fn parse_port(text: &str) -> Result<PortSpec, AddressRefusal> {
    if text.is_empty() {
        return Err(AddressRefusal::NoPort);
    }
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        return match text.parse::<u32>() {
            Ok(port) if port <= MAX_PORT => Ok(PortSpec::Literal(port)),
            _ => Err(AddressRefusal::PortRange(text.to_owned())),
        };
    }
    let mut characters = text.chars();
    let first = characters.next().expect("the text is not empty");
    if (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Ok(PortSpec::Scalar(text.to_owned()));
    }
    Err(AddressRefusal::PortShape(text.to_owned()))
}

/// Parse a dotted-quad IPv4 address.
///
/// # Errors
///
/// [`AddressRefusal::Hostname`] when the spelling is a name rather than an
/// address, and [`AddressRefusal::Ipv4Component`] when it is an address shape
/// with a component that is not one.
pub fn parse_ipv4(text: &str) -> Result<[u8; 4], AddressRefusal> {
    // A spelling with no digits at all, or with a letter anywhere, is a *name*
    // rather than a malformed address, and the refusal says so: that is the
    // difference between "you wrote this address wrongly" and "there is no name
    // resolution here".
    if text.is_empty() {
        return Err(AddressRefusal::Empty);
    }
    if text
        .bytes()
        .any(|byte| !byte.is_ascii_digit() && byte != b'.')
    {
        return Err(AddressRefusal::Hostname(text.to_owned()));
    }
    let components: Vec<&str> = text.split('.').collect();
    if components.len() != 4 {
        return Err(AddressRefusal::Ipv4Component(text.to_owned()));
    }
    let mut octets = [0u8; 4];
    for (slot, component) in octets.iter_mut().zip(components) {
        if component.is_empty() || component.len() > 3 {
            return Err(AddressRefusal::Ipv4Component(text.to_owned()));
        }
        if component.len() > 1 && component.starts_with('0') {
            return Err(AddressRefusal::Ipv4Component(text.to_owned()));
        }
        let Ok(value) = component.parse::<u16>() else {
            return Err(AddressRefusal::Ipv4Component(text.to_owned()));
        };
        let Ok(value) = u8::try_from(value) else {
            return Err(AddressRefusal::Ipv4Component(text.to_owned()));
        };
        *slot = value;
    }
    Ok(octets)
}

/// Parse an IPv6 address in this route's admitted textual subset.
///
/// # Errors
///
/// [`AddressRefusal`] naming which part of the spelling leaves the subset — with
/// the embedded-IPv4 form and a zone identifier refused by their own names
/// rather than as a generic shape error, because each is a capability decision
/// and not a typo.
pub fn parse_ipv6(text: &str) -> Result<[u8; 16], AddressRefusal> {
    if text.is_empty() {
        return Err(AddressRefusal::Empty);
    }
    if text.contains('%') {
        return Err(AddressRefusal::ZoneIdentifier(text.to_owned()));
    }
    if text.contains('.') {
        return Err(AddressRefusal::EmbeddedIpv4(text.to_owned()));
    }
    let (head, tail) = match text.split_once("::") {
        Some((head, tail)) => {
            if tail.contains("::") {
                return Err(AddressRefusal::Ipv6Shape(text.to_owned()));
            }
            (head, Some(tail))
        }
        None => (text, None),
    };
    let leading = parse_groups(head, text)?;
    let trailing = match tail {
        Some(tail) => parse_groups(tail, text)?,
        None => Vec::new(),
    };
    match tail {
        // No elision: exactly eight groups, no more and no fewer.
        None => {
            if leading.len() != 8 {
                return Err(AddressRefusal::Ipv6Shape(text.to_owned()));
            }
        }
        // One elision, standing for **one or more** all-zero groups, which is
        // what RFC 4291 §2.2 says it stands for. `0:0:0:0:0:0:0::1` therefore
        // refuses, because its elision would stand for none.
        Some(_) => {
            if leading.len() + trailing.len() >= 8 {
                return Err(AddressRefusal::Ipv6Shape(text.to_owned()));
            }
        }
    }
    let mut octets = [0u8; 16];
    for (index, group) in leading.iter().enumerate() {
        octets[index * 2] = (group >> 8) as u8;
        octets[index * 2 + 1] = (group & 0xff) as u8;
    }
    let offset = 8 - trailing.len();
    for (index, group) in trailing.iter().enumerate() {
        octets[(offset + index) * 2] = (group >> 8) as u8;
        octets[(offset + index) * 2 + 1] = (group & 0xff) as u8;
    }
    Ok(octets)
}

/// One side of an elision, or a whole uncompressed address.
fn parse_groups(text: &str, whole: &str) -> Result<Vec<u16>, AddressRefusal> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut groups = Vec::new();
    for group in text.split(':') {
        if group.is_empty() || group.len() > 4 {
            return Err(AddressRefusal::Ipv6Shape(whole.to_owned()));
        }
        let Ok(value) = u16::from_str_radix(group, 16) else {
            return Err(AddressRefusal::Ipv6Shape(whole.to_owned()));
        };
        groups.push(value);
        if groups.len() > 8 {
            return Err(AddressRefusal::Ipv6Shape(whole.to_owned()));
        }
    }
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dotted_quad_parses_to_its_octets() {
        assert_eq!(parse_ipv4("127.0.0.1"), Ok([127, 0, 0, 1]));
        assert_eq!(parse_ipv4("0.0.0.0"), Ok([0, 0, 0, 0]));
        assert_eq!(parse_ipv4("255.255.255.255"), Ok([255, 255, 255, 255]));
    }

    #[test]
    fn a_leading_zero_component_is_refused_rather_than_read() {
        assert_eq!(
            parse_ipv4("127.0.0.01"),
            Err(AddressRefusal::Ipv4Component("127.0.0.01".to_owned()))
        );
    }

    #[test]
    fn a_hostname_is_refused_as_a_name_and_not_as_a_malformed_address() {
        assert_eq!(
            parse_ipv4("example.com"),
            Err(AddressRefusal::Hostname("example.com".to_owned()))
        );
    }

    #[test]
    fn the_two_degenerate_ipv6_forms_parse() {
        assert_eq!(parse_ipv6("::"), Ok([0u8; 16]));
        let mut loopback = [0u8; 16];
        loopback[15] = 1;
        assert_eq!(parse_ipv6("::1"), Ok(loopback));
        assert!(IpAddress::V6(loopback).is_loopback());
        assert!(IpAddress::V6([0u8; 16]).is_unspecified());
    }

    #[test]
    fn a_compressed_and_an_uncompressed_spelling_are_one_address() {
        assert_eq!(
            parse_ipv6("::1"),
            parse_ipv6("0:0:0:0:0:0:0:1"),
            "comparison is on bytes, never on text"
        );
        assert_eq!(parse_ipv6("FE80::1"), parse_ipv6("fe80::1"));
    }

    #[test]
    fn the_embedded_ipv4_form_and_a_zone_are_refused_by_their_own_names() {
        assert_eq!(
            parse_ipv6("::ffff:127.0.0.1"),
            Err(AddressRefusal::EmbeddedIpv4("::ffff:127.0.0.1".to_owned()))
        );
        assert_eq!(
            parse_ipv6("fe80::1%eth0"),
            Err(AddressRefusal::ZoneIdentifier("fe80::1%eth0".to_owned()))
        );
    }

    #[test]
    fn an_elision_stands_for_at_least_one_group() {
        assert!(parse_ipv6("0:0:0:0:0:0:0::1").is_err());
        assert!(parse_ipv6("1:2:3:4:5:6:7").is_err());
        assert!(parse_ipv6("1:2:3:4:5:6:7:8:9").is_err());
        assert!(parse_ipv6("1:2:3:4:5:6:7:8").is_ok());
    }

    #[test]
    fn an_endpoint_carries_a_host_and_a_port() {
        let address = parse_socket_address("127.0.0.1:8080").expect("an address");
        assert_eq!(address.ip, IpAddress::V4([127, 0, 0, 1]));
        assert_eq!(address.port, PortSpec::Literal(8080));
        assert_eq!(address.family(), Family::V4);
        let address = parse_socket_address("[::1]:0").expect("an address");
        assert_eq!(address.family(), Family::V6);
        assert_eq!(address.port, PortSpec::Literal(0));
        let address = parse_socket_address("10.0.0.1:port").expect("an address");
        assert_eq!(address.port, PortSpec::Scalar("port".to_owned()));
    }

    #[test]
    fn a_bare_ipv6_host_asks_for_its_brackets() {
        assert_eq!(
            parse_socket_address("::1:8080"),
            Err(AddressRefusal::BracketsRequired("::1".to_owned()))
        );
    }

    #[test]
    fn a_port_out_of_range_refuses_rather_than_wrapping() {
        assert_eq!(
            parse_socket_address("127.0.0.1:65536"),
            Err(AddressRefusal::PortRange("65536".to_owned()))
        );
    }

    #[test]
    fn the_canonical_rendering_never_compresses() {
        assert_eq!(
            IpAddress::V6(parse_ipv6("::1").expect("an address")).to_string(),
            "0:0:0:0:0:0:0:1"
        );
        assert_eq!(IpAddress::V4([127, 0, 0, 1]).to_string(), "127.0.0.1");
    }
}
