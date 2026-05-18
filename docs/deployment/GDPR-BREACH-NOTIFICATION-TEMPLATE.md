# GDPR Data Breach Notification Template

Template for notifying supervisory authority within 72 hours of a personal data breach (GDPR Article 33).

---

## When to Use This Template

**Required when**:
- Personal data has been breached
- Breach likely to result in risk to rights and freedoms
- Within 72 hours of becoming aware

**Not required when**:
- Breach unlikely to result in risk (e.g., encrypted data, key not compromised)
- Technical measures render data unintelligible

---

## Pre-Notification Checklist

Before submitting notification:

- [ ] Breach confirmed (not false alarm)
- [ ] Scope assessed (what data, how many subjects)
- [ ] Risk level determined (high/medium/low)
- [ ] Containment actions taken
- [ ] Forensic evidence preserved
- [ ] Legal counsel consulted
- [ ] Internal incident commander assigned

---

## Notification Template

**To**: [Supervisory Authority - e.g., CNIL, ICO, etc.]  
**From**: Data Protection Officer / Security Team  
**Date**: [YYYY-MM-DD]  
**Subject**: Personal Data Breach Notification - [Incident ID]

---

### 1. Nature of the Personal Data Breach

**Date and time of breach**:
- First occurrence: [YYYY-MM-DD HH:MM UTC]
- Discovery: [YYYY-MM-DD HH:MM UTC]
- Notification: [YYYY-MM-DD HH:MM UTC] (within 72 hours: ☐ Yes ☐ No)

**Type of breach**:
- ☐ Confidentiality breach (unauthorized access/disclosure)
- ☐ Integrity breach (unauthorized alteration)
- ☐ Availability breach (loss of access/data)

**Description of breach**:
> [Detailed description of what happened]
>
> Example for FILE Relay Server:
> On [date] at [time] UTC, an unauthorized party gained access to the relay server's IP hash secret through [attack vector]. This secret is used to hash client IP addresses for rate limiting. The attacker could theoretically reverse IP addresses from observed hashes if they captured network traffic during the exposure window.

**Categories of personal data affected**:
- ☐ IP addresses (hashed)
- ☐ Connection metadata
- ☐ Other: [specify]

**Number of data subjects affected**:
- Confirmed: [N] individuals
- Estimated: [M-N] individuals (if exact number unknown)

**Geographic scope**:
- ☐ EU/EEA only
- ☐ Global
- ☐ Specific countries: [list]

---

### 2. Name and Contact Details of Data Protection Officer

**Data Protection Officer**:
- Name: [Full Name]
- Email: dpo@file-network.example
- Phone: [+XX XXX XXX XXXX]
- Address: [Full postal address]

**Alternative contact** (if DPO unavailable):
- Name: [Full Name]
- Email: security@file-network.example
- Phone: [+XX XXX XXX XXXX]

---

### 3. Likely Consequences of the Breach

**Risk assessment**:

**Confidentiality impact**:
> [Describe what data was exposed and to whom]
>
> Example:
> IP addresses were hashed using HMAC-SHA256 with a per-boot secret. The secret was exposed for approximately [X] hours. An attacker with network captures from this period could theoretically reverse the hashes to obtain IP addresses.

**Availability impact**:
> [If service was disrupted, describe impact on data subjects]
>
> Example:
> The relay server was taken offline for [X] hours during incident response. Users could not connect during this period but no data was lost due to the stateless design.

**Integrity impact**:
> [If data was altered, describe the extent]
>
> Example:
> No data was altered. The breach was limited to potential IP address exposure.

**Risk to rights and freedoms**:
- **Risk level**: ☐ Low ☐ Medium ☐ High
- **Justification**:
  > [Explain why this risk level]
  >
  > Example:
  > Risk assessed as **Low** because:
  > 1. Only IP addresses potentially exposed (no names, emails, or sensitive data)
  > 2. IP addresses already dynamic (many ISPs rotate IPs)
  > 3. No financial, health, or government data involved
  > 4. Exposure window limited to [X] hours
  > 5. No evidence of actual reverse-engineering of hashes

**Affected individuals notified**:
- ☐ Yes (date: [YYYY-MM-DD])
- ☐ No (justification: low risk, disproportionate effort)
- ☐ Pending (expected: [YYYY-MM-DD])

---

### 4. Measures Taken to Address the Breach

**Immediate containment** (within first hour):
1. [Action taken, timestamp]
   > Example: [14:23 UTC] Relay server taken offline
2. [Action taken, timestamp]
   > Example: [14:35 UTC] IP hash secret rotated
3. [Action taken, timestamp]
   > Example: [14:50 UTC] All active sessions terminated

**Investigation** (first 24 hours):
1. [Action taken, timestamp]
   > Example: [15:30 UTC] Forensic analysis initiated
2. [Action taken, timestamp]
   > Example: [17:00 UTC] Attack vector identified (CVE-2024-XXXXX)
3. [Action taken, timestamp]
   > Example: [20:00 UTC] Full scope determined

**Remediation** (within 72 hours):
1. [Action taken, timestamp]
   > Example: [Day 1, 22:00 UTC] Security patch applied
2. [Action taken, timestamp]
   > Example: [Day 2, 10:00 UTC] Independent security audit completed
3. [Action taken, timestamp]
   > Example: [Day 3, 14:00 UTC] Service restored with enhanced monitoring

**Long-term measures**:
1. [Planned action, due date]
   > Example: [2024-02-15] Implement secret rotation every 24 hours
2. [Planned action, due date]
   > Example: [2024-02-20] Add intrusion detection system
3. [Planned action, due date]
   > Example: [2024-03-01] Complete penetration test

**Communication**:
- Internal: [How internal teams were notified]
- Customers: [How customers were notified, if applicable]
- Public: [If public disclosure made, provide link]

---

### 5. Technical Details (for Authority Review)

**System architecture**:
> [Brief overview of the affected system]
>
> Example:
> FILE Relay Server is a stateless UDP relay for peer-to-peer connections. It hashes client IP addresses for rate limiting using HMAC-SHA256 with a per-boot secret stored in memory only.

**Security controls in place**:
- ✅ IP hashing with HMAC-SHA256
- ✅ Per-boot secret rotation
- ✅ No persistent storage of IP addresses
- ✅ 5-minute automatic peer timeout
- ✅ CAPoW DoS protection
- ✅ Rate limiting

**Attack vector**:
> [Detailed technical description of how breach occurred]
>
> Example:
> Attacker exploited [vulnerability] in [component] to gain access to [system]. The IP hash secret was stored in [location] and was accessible via [method].

**Affected infrastructure**:
- Servers: [List of affected servers/regions]
- Timeframe: [Start] to [End]
- Number of requests during exposure: [N]

**Forensic evidence preserved**:
- ✅ Server logs (full 7-day retention)
- ✅ Network packet captures
- ✅ Memory dumps
- ✅ Configuration snapshots
- ✅ Audit logs

---

### 6. Cross-Border Data Flows

**Data subjects in multiple jurisdictions**:
- ☐ Yes → Additional notifications to: [List authorities]
- ☐ No

**Lead supervisory authority** (GDPR Article 56):
- Name: [Authority name]
- Country: [Country]
- Reason: Main establishment in [location]

---

### 7. Supporting Documentation

**Attached documents**:
1. Incident timeline (detailed)
2. Forensic analysis report
3. Risk assessment
4. Communication to affected individuals (if sent)
5. Independent security audit report

**Available upon request**:
- Full server logs
- Network captures
- Configuration files
- Source code review

---

### 8. Declaration

I hereby declare that the information provided in this notification is accurate and complete to the best of my knowledge.

**Name**: [Full Name]  
**Position**: [Data Protection Officer / Security Lead]  
**Date**: [YYYY-MM-DD]  
**Signature**: [Digital or scanned signature]

---

## Post-Notification Actions

After submitting notification:

- [ ] Await acknowledgment from supervisory authority (5 working days)
- [ ] Respond to any follow-up questions promptly
- [ ] Continue investigation and remediation
- [ ] Complete post-mortem (see `POST-MORTEM-TEMPLATE.md`)
- [ ] Update incident response procedures
- [ ] Conduct root cause analysis
- [ ] Implement preventive measures
- [ ] Schedule follow-up with authority (if required)

---

## Authority Contact Information

### EU/EEA Supervisory Authorities

**France (CNIL)**:
- Email: notifications@cnil.fr
- Phone: +33 1 53 73 22 22
- Portal: https://www.cnil.fr/

**Ireland (DPC)**:
- Email: info@dataprotection.ie
- Phone: +353 57 868 4800
- Portal: https://forms.dataprotection.ie/

**Germany (BfDI)**:
- Email: poststelle@bfdi.bund.de
- Phone: +49 228 997799-0
- Portal: https://www.bfdi.bund.de/

**UK (ICO)**:
- Email: casework@ico.org.uk
- Phone: +44 303 123 1113
- Portal: https://ico.org.uk/make-a-complaint/data-protection-complaints/

[Full list of EU supervisory authorities](https://edpb.europa.eu/about-edpb/about-edpb/members_en)

---

## Example: FILE Relay Server IP Hash Secret Breach

**Scenario**: IP hash secret leaked through vulnerability, attacker could reverse hashes if they captured traffic.

**Nature of breach**:
> On 2024-01-15 at 14:23 UTC, an unauthorized party exploited a vulnerability in the metrics endpoint to extract the IP hash secret from server memory. This secret is used to hash client IP addresses using HMAC-SHA256 for rate limiting purposes. The attacker could theoretically reverse IP addresses from observed hashes if they captured network traffic during the 6-hour exposure window (14:23-20:15 UTC). The server was taken offline at 14:35 UTC and the secret was rotated immediately.

**Personal data affected**:
> IP addresses (hashed) of approximately 5,000 users who connected during the 6-hour window. No names, emails, or other identifying information was stored or exposed.

**Risk assessment**:
> Risk assessed as **Low** because:
> 1. Only IP addresses potentially exposed (no names or sensitive data)
> 2. Attacker would need both the leaked secret AND network captures from the same period
> 3. No evidence of packet capture by the attacker
> 4. IP addresses are dynamic (most ISPs rotate IPs within 24 hours)
> 5. Exposure window limited to 6 hours
> 6. Secret rotated immediately upon discovery

**Measures taken**:
> 1. [14:35 UTC] Relay server taken offline (12 minutes after discovery)
> 2. [14:40 UTC] IP hash secret rotated
> 3. [14:50 UTC] All active sessions terminated
> 4. [15:30 UTC] Vulnerability patched (metrics endpoint access control)
> 5. [17:00 UTC] Independent security audit completed
> 6. [20:15 UTC] Service restored with enhanced monitoring
> 7. [Day 2] Secret rotation automated (every 24 hours)
> 8. [Day 3] Intrusion detection system deployed

**Notification to individuals**: Not required (low risk, disproportionate effort to identify and contact based on hashed IPs alone)

---

## Further Reading

- **GDPR Article 33**: [Notification of breach to supervisory authority](https://gdpr-info.eu/art-33-gdpr/)
- **GDPR Article 34**: [Communication of breach to data subject](https://gdpr-info.eu/art-34-gdpr/)
- **EDPB Guidelines**: [Guidelines on Personal Data Breach Notification](https://edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-072021-notification-personal-data-breach-under_en)

---

**Emergency contact**: security@file-network.example | +XX XXX XXX XXXX
