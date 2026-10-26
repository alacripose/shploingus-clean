//! **A ticket-scoped grant: the one thing an agent may present to act, and it is signed.**
//!
//! `voxel_model::grant::Grant` already owns *who authorised what, bounded* — a `granter` and a
//! `scope`, both required, neither defaulted. This crate does not restate that type; it **wraps** it
//! with the three facts a ticket-scoped act needs and the one that makes it checkable:
//!
//! | field | what it bounds |
//! |---|---|
//! | `ticket` | the one ticket this grant authorizes. A grant for another ticket is refused by name |
//! | `segment` / `axis` | the one measured cell the act is about |
//! | `expires` | the tick the grant stops being authority at. **Exclusive**: `now == expires` is expired |
//! | `signature` | the security layer's signature over all of the above, so a holder cannot widen it |
//!
//! # The signature covers the fields a reader sees
//!
//! [`TicketGrant::payload`] is a domain-separated encoding (`voxel-axis-mcp/ticket-grant`) of every
//! field, length-prefixed by [`Identity::of_fields`]. Editing the ticket, the cell or the expiry after
//! signing is [`RefusalKind::SignatureInvalid`]; signing with a key this verifier does not hold is
//! [`RefusalKind::UntrustedKey`]. Both are the security crate's own refusals, reused rather than
//! restated.

use voxel_axis_matrix::Segment;
use voxel_axis_security::Security;
use voxel_model::axis::AxisKind;
use voxel_model::grant::Grant;
use voxel_model::refusal::{RefusalKind, RefusalSet};
use voxel_model::Identity;
use voxel_package::signing::{KeyId, Signer, Signature, Verifier};

/// The domain tag of a grant's signed bytes. A second consumer with this tag would be a second
/// interpretation of the same bytes.
const TAG: &str = "voxel-axis-mcp/ticket-grant";

/// The tag a grant carries **on the wire**: one tab-separated line, so a credential can cross a
/// socket without a serialisation library and without a second encoding of the signed fields.
///
/// ```text
/// axis-grant/1<TAB>granter<TAB>scope<TAB>ticket<TAB>segment<TAB>axis<TAB>expires<TAB>algorithm<TAB>key-id<TAB>hex-signature
/// ```
///
/// The wire form is not verified on parse: [`TicketGrant::from_wire`] reconstructs the fields and
/// [`TicketGrant::verify`] recomputes the payload over them, so a wire line is a **claim** until the
/// signature over exactly those bytes holds.
pub const WIRE_TAG: &str = "axis-grant/1";
/// The fields on a wire line, tag included.
pub const WIRE_FIELDS: usize = 10;

/// A grant to perform one act on one ticket's measured cell, signed by the machine's key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketGrant {
    grant: Grant,
    ticket: String,
    segment: Segment,
    axis: AxisKind,
    expires: u64,
    signature: Signature,
}

impl TicketGrant {
    /// **Issue and sign.** The ticket must be named: a grant for no ticket authorizes nothing, and
    /// `Grant::new` refuses the empty granter or scope before anything is signed.
    pub fn issue(
        security: &Security,
        ticket: &str,
        segment: Segment,
        axis: AxisKind,
        expires: u64,
    ) -> Result<TicketGrant, RefusalSet> {
        if ticket.trim().is_empty() {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                "a ticket grant for no ticket is not a ticket grant: name the ticket it authorizes"
                    .to_string(),
            );
            return Err(refusals);
        }
        if ticket.contains(['\t', '\n', '\r']) {
            return Err(malformed(
                "a ticket that carries a tab or a newline cannot be written on the wire's one line, \
                 and a grant that cannot be written cannot be presented"
                    .to_string(),
            ));
        }
        let grant = Grant::new(
            security.key_id().as_str(),
            format!("ticket {ticket} — {}/{}", segment.name(), axis.name()),
        )?;
        let signature = security.sign(&payload_of(&grant, ticket, segment, axis, expires))?;
        Ok(TicketGrant {
            grant,
            ticket: ticket.to_string(),
            segment,
            axis,
            expires,
            signature,
        })
    }

    /// The ticket this authorizes, and no other.
    pub fn ticket(&self) -> &str {
        &self.ticket
    }

    /// The one cell this authorizes.
    pub fn cell(&self) -> (Segment, AxisKind) {
        (self.segment, self.axis)
    }

    /// The tick it stops being authority at. **Exclusive.**
    pub fn expires(&self) -> u64 {
        self.expires
    }

    /// The attributed, bounded half — `voxel_model::grant::Grant`, unchanged.
    pub fn grant(&self) -> &Grant {
        &self.grant
    }

    /// The security layer's signature over [`TicketGrant::payload`].
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// **The credential as one line**, for a socket that carries text.
    pub fn to_wire(&self) -> String {
        format!(
            "{WIRE_TAG}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.grant.granter(),
            self.grant.scope(),
            self.ticket,
            self.segment.name(),
            self.axis.name(),
            self.expires,
            self.signature.algorithm(),
            self.signature.key_id().as_str(),
            hex(self.signature.bytes())
        )
    }

    /// **Read a credential off the wire.** Parse only: every field is reconstructed and the
    /// signature travels with it, but nothing here says the signature holds — [`TicketGrant::verify`]
    /// is that, and [`crate::Registry::apply`] calls it before any field is used.
    pub fn from_wire(text: &str) -> Result<TicketGrant, RefusalSet> {
        let fields: Vec<&str> = text.split('\t').collect();
        if fields.len() != WIRE_FIELDS || fields[0] != WIRE_TAG {
            return Err(malformed(format!(
                "a credential is {WIRE_TAG} and {WIRE_FIELDS} tab-separated fields; this line has {} \
                 field(s) and starts {:?}",
                fields.len(),
                fields.first().copied().unwrap_or_default()
            )));
        }
        if fields
            .iter()
            .any(|field| field.contains('\n') || field.contains('\r'))
        {
            return Err(malformed(
                "a credential is one line, and one of its fields carries a newline".to_string(),
            ));
        }
        let segment = Segment::parse(fields[4]).map_err(|refusal| {
            malformed(format!("the segment field is not a segment: {refusal}"))
        })?;
        let axis = AxisKind::ALL
            .into_iter()
            .find(|axis| axis.name() == fields[5])
            .ok_or_else(|| {
                malformed(format!(
                    "{:?} is not an axis; the vocabulary is closed",
                    fields[5]
                ))
            })?;
        let expires = fields[6].parse::<u64>().map_err(|_| {
            malformed(format!(
                "the expiry field {:?} is not an unsigned tick",
                fields[6]
            ))
        })?;
        let algorithm = fields[7].parse::<u16>().map_err(|_| {
            malformed(format!(
                "the algorithm field {:?} is not a number",
                fields[7]
            ))
        })?;
        let key_id = KeyId::parse(fields[8]).map_err(malformed)?;
        let bytes = unhex(fields[9]).ok_or_else(|| {
            malformed(format!(
                "the signature field {:?} is not an even-length lowercase hex string",
                fields[9]
            ))
        })?;
        Ok(TicketGrant {
            grant: Grant::new(fields[1], fields[2])?,
            ticket: fields[3].to_string(),
            segment,
            axis,
            expires,
            signature: Signature::new(algorithm, key_id, bytes),
        })
    }

    /// **Is this signature over these fields by a key this verifier holds?** The payload is
    /// recomputed from the fields a reader sees, so a widened copy is refused.
    pub fn verify(&self, security: &Security) -> Result<(), RefusalSet> {
        security.verify(&self.payload(), &self.signature)
    }

    /// The line a console or a receipt prints.
    pub fn render(&self) -> String {
        format!(
            "grant {} for {} on {}/{} until tick {} — signed by {}",
            self.grant.granter(),
            self.ticket,
            self.segment.name(),
            self.axis.name(),
            self.expires,
            self.signature.key_id().as_str()
        )
    }

    /// Every field, domain-separated, in one canonical identity.
    fn payload(&self) -> Vec<u8> {
        payload_of(
            &self.grant,
            &self.ticket,
            self.segment,
            self.axis,
            self.expires,
        )
    }
}

/// A refusal for a line that is not a credential. The kind is the record's own: a malformed record
/// is not an untrusted key, and collapsing the two would tell a reader the signature failed when
/// nothing was ever checked.
fn malformed(detail: String) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(RefusalKind::MalformedRecord, detail);
    refusals
}

/// Lowercase hex, fixed width, because that is what [`unhex`] accepts and what the rest of this
/// project writes.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode lowercase hex, refusing odd lengths and anything outside `0-9a-f`.
fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect()
}

/// The signed bytes, written from the fields a reader sees — the same call from issue and from
/// verify, so a widened copy cannot be signed and checked over different encodings.
fn payload_of(
    grant: &Grant,
    ticket: &str,
    segment: Segment,
    axis: AxisKind,
    expires: u64,
) -> Vec<u8> {
    Identity::of_fields(
        TAG,
        &[
            ticket.as_bytes(),
            segment.name().as_bytes(),
            axis.name().as_bytes(),
            expires.to_string().as_bytes(),
            grant.granter().as_bytes(),
            grant.scope().as_bytes(),
        ],
    )
    .to_string()
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> (PathBuf, Security) {
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-mcp-grant-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("checkout")).expect("a scratch checkout");
        let security = Security::open(&root.join("checkout"), root.join("keys/signing.key"))
            .expect("a machine key");
        (root, security)
    }

    #[test]
    fn a_grant_is_named_scoped_and_holds_for_the_key_that_signed_it() {
        let (root, security) = scratch("holds");
        let grant = TicketGrant::issue(&security, "T-0954", Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("a signed grant");
        assert_eq!(grant.ticket(), "T-0954");
        assert_eq!(grant.cell(), (Segment::Shell, AxisKind::EvidenceGrade));
        assert_eq!(grant.expires(), 40);
        assert_eq!(grant.grant().granter(), security.key_id().as_str());
        assert!(
            grant.render().contains("T-0954") && grant.render().contains("shell/evidence_grade"),
            "{}",
            grant.render()
        );
        grant.verify(&security).expect("its own key and bytes");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_grant_widened_after_signing_is_refused_by_the_signature() {
        let (root, security) = scratch("widened");
        let mut grant = TicketGrant::issue(&security, "T-0954", Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("a signed grant");
        grant.ticket = "T-9999".to_string();
        let refused = grant.verify(&security).expect_err("the ticket is covered by the signature");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid),
            "{refused:?}"
        );
        let (other_root, other) = scratch("widened-other");
        let grant = TicketGrant::issue(&security, "T-0954", Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("a signed grant");
        assert!(grant
            .verify(&other)
            .expect_err("another key")
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::UntrustedKey));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&other_root);
    }

    #[test]
    fn a_grant_round_trips_through_its_wire_line_and_still_verifies() {
        let (root, security) = scratch("wire");
        let grant = TicketGrant::issue(&security, "T-0954", Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("a signed grant");
        let wire = grant.to_wire();
        assert_eq!(wire.split('\t').count(), WIRE_FIELDS, "{wire}");
        assert!(wire.starts_with(WIRE_TAG), "{wire}");
        let read_back = TicketGrant::from_wire(&wire).expect("its own line parses");
        assert_eq!(read_back, grant, "every field survives the wire");
        read_back.verify(&security).expect("and the signature still holds");

        let tampered = wire.replace("T-0954", "T-9999");
        assert_ne!(tampered, wire);
        let moved = TicketGrant::from_wire(&tampered).expect("the line still parses");
        assert!(
            moved
                .verify(&security)
                .expect_err("the ticket is covered by the signature")
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid)
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_line_that_is_not_a_credential_is_refused_as_a_malformed_record() {
        for line in [
            "",
            "axis-grant/1",
            "axis-grant/1\tthe user\tscope\tT-0954\tshell",
            "axis-grant/2\tthe user\tscope\tT-0954\tshell\tevidence_grade\t40\t1\tsha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\t00",
            "axis-grant/1\tthe user\tscope\tT-0954\tnowhere\tevidence_grade\t40\t1\tsha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\t00",
            "axis-grant/1\tthe user\tscope\tT-0954\tshell\tevidence_grade\tsoon\t1\tsha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\t00",
            "axis-grant/1\tthe user\tscope\tT-0954\tshell\tevidence_grade\t40\t1\tsha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\tzz",
        ] {
            let refused = TicketGrant::from_wire(line).expect_err(line);
            assert!(
                refused
                    .all()
                    .iter()
                    .any(|refusal| refusal.kind() == RefusalKind::MalformedRecord),
                "{line}: {refused:?}"
            );
        }
    }

    #[test]
    fn a_ticket_that_would_break_the_wire_line_is_refused_at_issue() {
        let (root, security) = scratch("wire-tab");
        let refused = TicketGrant::issue(
            &security,
            "T-0954\tand another",
            Segment::Shell,
            AxisKind::EvidenceGrade,
            40,
        )
        .expect_err("a tab in the ticket cannot be written on one line");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_grant_for_no_ticket_refuses_construction_rather_than_signing_nothing() {
        let (root, security) = scratch("empty");
        let refused = TicketGrant::issue(&security, "   ", Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect_err("no ticket");
        assert_eq!(refused.all()[0].kind(), RefusalKind::FabricatedFeature);
        let _ = std::fs::remove_dir_all(&root);
    }
}
