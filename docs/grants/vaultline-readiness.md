# Vaultline grant readiness

Source checked October 11, 2026: [official listing](https://superteam.fun/earn/listing/vaultline-security-audit-grants-for-colosseum-projects-up-to-4000-usdc-each).

Applications close **October 31, 2026**; announcements are scheduled for **November 2, 2026**. The offer covers up to **4,000 USDC of audit services**, not cash. Ten projects are selected. Confirm the platform's exact closing time before submission.

| Requirement | Candidate evidence / remaining action |
| --- | --- |
| Colosseum participant | **Not submitted, confirmed by applicant October 11.** A profile URL is not a project submission. Register and submit an eligible project before claiming participation; sponsor acceptance of the current event remains unconfirmed. |
| Rights and attribution | The repository has a proprietary license. Preserve copyright and obtain permission for use and submission; public visibility alone is insufficient. |
| Mature, stable code | SOL vault candidate, tests, SDK and scope manifest prepared locally; freeze commit after owner review. |
| Accessible repository | Public upstream exists; **this branch must be published** before reviewers can access its changes. |
| Audit program and approximate LOC | `docs/AUDIT-SCOPE.md` and `docs/audit-manifest.json`. |
| Stage, users, TVL | New code tested locally; owner must confirm deployment and current metrics with evidence. |
| Project description and audit rationale | Prepared in the private local `GRANT-APPLICATION.md` draft. |
| Team contact and remediation capacity | **Owner contact and availability required.** |
| Quote above grant | Owner must confirm willingness/budget for any excess quote before applying. |
| Selected-team publicity | Owner must accept sponsor naming and post-sharing terms. |

Selection considers project potential, dependence on secure code, and readiness. The strongest evidence here is a small, inspectable custody boundary plus reproducible transfer/rejection/recovery tests. Do not substitute roadmap promises, unsupported adoption figures, or test counts for eligibility and deployed-version evidence.

## Colosseum entry prerequisite

Checked October 11: the signed-in dashboard offers registration for Crypto World's Fair. Registration and submissions close October 12, 2026 at 11:59 p.m. PDT, equivalent to **October 13 at 12:29 p.m. IST**. Registration alone does not submit a project. See the [official rules](https://colosseum.com/legal/Crypto%20World's%20Fair%20Hackathon%20Rules.pdf) and [submission FAQ](https://colosseum.com/hackathon#faqs). The entry must disclose relevant prior development; this project predates the event. Keep the inherited baseline distinct from work completed during the competition. Never describe this proprietary repository as open source or claim sole authorship of inherited work.

The private local `SUBMISSION.md` contains proposed entry copy, video scripts and missing owner facts. Confirm personal eligibility and accept the rules before registration. An actual entry, accessible candidate, videos and accurate team/business facts remain necessary. The grant listing does not identify a specific Colosseum event, so a new Crypto World's Fair entry must not be represented as confirmed grant eligibility without sponsor clarification.

## Publication checklist

- Review the candidate diff and native-SOL scope. No SPL or arbitrary-protocol support should be claimed.
- Run `scripts/verify.sh` from the committed candidate, and update verification evidence after changes.
- Publish the branch and record the exact reviewed commit in the application. Keep contact and budget details private unless intended for publication.
- Confirm the actual Colosseum link, team contact, user/TVL measurements, deployment version and authority.
- Submit through the official application form once all owner fields are complete. No application was submitted by this preparation work.
