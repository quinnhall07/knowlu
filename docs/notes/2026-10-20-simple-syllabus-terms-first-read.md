# Simple Syllabus terms: first read (flags only)

Date read: 2026-10-20. This is L5 task T18 (plan `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md`,
spec `docs/specs/2026-10-05-syllabus-class-times-design.md` §15.6). It used public pages only, with no
sign-in and no student data.

*Flags for counsel, not a legal opinion; counsel reads these with the privacy page before the Pilot.*

## What was read

The vendor is Simple Systems Group, LLC, doing business as Simple Higher Ed. Its product is Simple Syllabus.

| Document | URL | How read | Stated date |
|---|---|---|---|
| Terms of Service | `https://ua.simplesyllabus.com/en-US/terms-of-service` | Rendered browser DOM, 2026-10-20 (headless, fresh profile, no sign-in) | "Last updated August 30, 2026" |
| Privacy Policy | `https://ua.simplesyllabus.com/en-US/privacy-policy` | Rendered browser DOM, 2026-10-20 (same) | "Last updated April 22, 2025" |
| Marketing privacy page (secondary) | `https://simplehighered.com/simple-higher-ed-and-privacy/` | Page summary only | none stated |

- Both pages render only with JavaScript; a plain fetch returns an empty shell titled "Simple Syllabus".
- `simplesyllabus.com/terms-of-service`, `/privacy-policy` and `simplehighered.com/terms-of-service/` return 404.
- The terms name no institution, so they read as the same documents the vendor serves on every school's subdomain.
- The Trust Center (`app.vanta.com/simplesyllabus.com/trust/…`) loads only with JavaScript and was not read.

## The twelve clauses (§15.6)

Section numbers are those in the page text. "ToS" is the terms of service and "PP" is the privacy policy.

**1. Automated access: flag.**
- ToS §4 opens "When using Simple Higher Ed products, you agree not to:" and lists this item: "Use any robot, spider, scraper, web crawler, or other automated means or interfaces to access the Services, extract data, or harvest User Content (including course materials and syllabi) without our express written permission."
- The bar is on any "automated means", not only on load, and it names syllabi.
- A script reading the student's own page is automated, whoever's session it runs in, and the text has no carve-out for a user's own pages.

**2. Interfaces: flag.**
- ToS §4 also bars "Reverse-engineer or attempt to extract the source code of Simple Syllabus, Simple Prep, or Simple TA."
- That is about source code, which an API call does not touch.
- Calling the document endpoint from a script falls under clause 1's "automated means or interfaces".

**3. Licence scope: flag.**
- ToS §2.1: "You are granted a limited, non-exclusive, non-transferable license to use the platform in accordance with these Terms."
- There is no "personal" or "non-commercial" wording.
- "In accordance with these Terms" ties the licence to §4. Whether a tool acting under the user's session is inside it turns on clause 1.

**4. Agents and third parties: no plain bar.**
- No clause covers third-party access or sharing access. Clause 1's "automated means" is the closest.
- Counsel should say whether the in-session read is "access by Knowlu".

**5. Content and copying: no plain bar on a student's copy.**
- ToS §2.2: "You retain all ownership and intellectual property rights in and to any syllabi, course materials... or other content you create, upload, or post using our Services ("User Content")." It adds that those rights "may be owned by you personally, or by your employing educational institution".
- That covers whoever posts content. It is silent on a student who keeps a copy.
- The one related bar is clause 1's "harvest User Content (including course materials and syllabi)" by automated means.

**6. Sending content on: silent.**
- ToS §2.3 binds the vendor, not the user: "We will never sell, monetize, or use your User Content to train public AI models."
- Nothing bars a user from sending content on.

**7. The institution's agreement: flag.**
- ToS §5: "Upon termination of your account or your institution's master agreement, your right to use the Services will immediately cease."
- ToS §3 has the vendor process student data on the institution's behalf, as a "School Official".
- So the terms sit under a school master agreement, which is not public (see below).

**8. Rate and load: flag.**
- ToS §4 bars any action that "imposes, or may impose (at our sole discretion), an unreasonable or disproportionately large load on our infrastructure", including "maliciously exceeding API rate limits".
- It gives no number, and the test is the vendor's.
- Offer A reads at most 16 documents a Connect, with the student present.

**9. Suspension and liability: flag.**
- ToS §5 gives only "your right to use the Services will immediately cease"; there is no suspension or notice procedure.
- ToS §6.2 limits the vendor's liability.
- The vendor could cut a student's access without warning, so the Connect screen must say what Knowlu does.

**10. Changes: flag.**
- The terms have no change clause, only the "Last updated" line.
- PP §12: the update "will be effective as soon as it is accessible", and the vendor "may notify you" of material changes.
- Notice is optional, so repeat this read by checking the two dates.

**11. Privacy policy: silent on scripts.**
- PP §1 says the vendor automatically collects "Log and Usage Data... pages and files viewed, searches and other actions you take", plus IP address, browser type and location data.
- An in-page read adds requests to those logs, and so does the student's own page view.
- The policy does not treat a script differently.

**12. Governing law and minors: flag.**
- ToS §7: Florida law, with actions "brought exclusively in the state or federal courts located in Hillsborough County, Florida."
- PP §8: "We do not knowingly collect data from or market to children under 13 years of age." Under-18 users come up only in the California removal right (PP §11).

## The school-contract finding

None public. A web search found no University of Alabama contract, procurement record, Board of Trustees item or acceptable-use text naming the vendor. These are the only public UA pages, and they describe the service only:

- `https://cit.ua.edu/technologies/simple-syllabus/`
- `https://bama.atlassian.net/wiki/spaces/CIT/pages/2213183489/Simple+Syllabus` (launch April 2023)
- `https://procurementcontracts.ua.edu/?p=91` (general contract guidance)

ToS §5 refers to "your institution's master agreement", so one exists. A public-records request to UA procurement is the next step, and it is counsel's call.

## Q39, second condition

**Not met.** ToS §4 plainly bars a scripted read:

> Use any robot, spider, scraper, web crawler, or other automated means or interfaces to access the Services, extract data, or harvest User Content (including course materials and syllabi) without our express written permission.

It does not exempt a student's own in-session read. Counsel decides how far it reaches; the vendor's written permission would remove the bar.
