<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# NOTICE — Loran

This NOTICE accompanies the GPL-3.0-or-later license that governs Loran. In any conflict between this NOTICE and the [LICENSE](LICENSE), the LICENSE text governs.

## No warranty

Loran is provided **"AS IS", without warranty of any kind**, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, title, and non-infringement. The entire risk as to the quality and performance of Loran is with you. Should Loran prove defective, you assume the cost of all necessary servicing, repair, or correction.

This restates and is bound by sections 15 and 16 of the GNU General Public License v3.

## No liability

In no event, unless required by applicable law or agreed to in writing, will the maintainer, copyright holder, or any other party who modifies and/or conveys Loran be liable to you for damages, including any general, special, incidental, or consequential damages arising out of the use or inability to use Loran (including but not limited to loss of data or data being rendered inaccurate, losses sustained by you or third parties, or a failure of Loran to operate with any other programs), even if such holder or other party has been advised of the possibility of such damages.

This restates and is bound by section 16 of the GNU General Public License v3.

## Posture

Loran is a personal hobby project per [Spacecraft Software Standard v1.1 §5.1](https://SpacecraftSoftware.org/standard/). There is no service-level commitment, no guaranteed response time, no support channel beyond best-effort communication with the maintainer, and no roadmap obligation. Forks are encouraged. Contributions are welcome but acceptance is at the maintainer's sole discretion (Standard §5.4).

## Trademarks

"Spacecraft Software" and "Loran" (in the project sense) refer to artifacts maintained by Mohamed Hammad. No trademark rights are claimed beyond the goodwill associated with the open-source release.

## Cryptographic trust

Upstream tarballs distributed by the Loran publisher pipeline (Phase 2 and later) are signed with minisign + ed25519. The publisher's public key is baked into each released binary via `include_bytes!` and is the sole trust root for tarball verification. Key rotation requires a new Loran release; downstream verification of tarballs fetched by an older binary against a rotated key will fail by design.

---

*Forged in Spacecraft Software.*
