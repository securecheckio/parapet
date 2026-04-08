use crate::output::event::{TransactionEvent, TransactionOutcome};
use crate::output::formatter::OutputFormatter;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

/// ISO 20022 format for SWIFT/banking integration
/// This is a simplified pain.001 (Customer Credit Transfer) message
pub struct Iso20022Formatter;

#[async_trait]
impl OutputFormatter for Iso20022Formatter {
    fn format_event(&self, event: &TransactionEvent) -> Result<Vec<u8>> {
        // Only format allowed/successful transactions for banking integration
        if !matches!(event.outcome, TransactionOutcome::Allowed) {
            return Ok(Vec::new());
        }

        let msg_id = &event.event_id;
        let creation_time = Utc::now().format("%Y-%m-%dT%H:%M:%S");

        // Extract amount (default to 0 if not present)
        let amount = event
            .amount
            .as_ref()
            .and_then(|a| a.split_whitespace().next())
            .and_then(|a| a.parse::<f64>().ok())
            .unwrap_or(0.0);

        let default_currency = "SOL".to_string();
        let currency = event.tokens.first().unwrap_or(&default_currency);

        // Simplified ISO 20022 XML structure
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.03">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>{}</MsgId>
      <CreDtTm>{}</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <CtrlSum>{:.9}</CtrlSum>
      <InitgPty>
        <Nm>SecureCheck</Nm>
        <Id>
          <OrgId>
            <Othr>
              <Id>{}</Id>
              <SchmeNm>
                <Cd>SOLA</Cd>
              </SchmeNm>
            </Othr>
          </OrgId>
        </Id>
      </InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>{}</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <ReqdExctnDt>{}</ReqdExctnDt>
      <Dbtr>
        <Nm>{}</Nm>
        <Id>
          <OrgId>
            <Othr>
              <Id>{}</Id>
            </Othr>
          </OrgId>
        </Id>
      </Dbtr>
      <CdtTrfTxInf>
        <PmtId>
          <EndToEndId>{}</EndToEndId>
          <TxId>{}</TxId>
        </PmtId>
        <Amt>
          <InstdAmt Ccy="{}">{:.9}</InstdAmt>
        </Amt>
        <Cdtr>
          <Nm>{}</Nm>
        </Cdtr>
        <RmtInf>
          <Ustrd>{}</Ustrd>
        </RmtInf>
        <SplmtryData>
          <Envlp>
            <RiskScore>{}</RiskScore>
            <RiskLevel>{}</RiskLevel>
            <Protocol>{}</Protocol>
            <Signature>{}</Signature>
          </Envlp>
        </SplmtryData>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>
"#,
            msg_id,
            creation_time,
            amount,
            event.wallet,
            event.event_id,
            creation_time,
            event.identity.as_deref().unwrap_or("Unknown"),
            event.wallet,
            event.event_id,
            event.signature.as_deref().unwrap_or(""),
            currency,
            amount,
            event.destination.as_deref().unwrap_or("Unknown"),
            event.summary,
            event.risk_score,
            event.risk_level.as_str(),
            event.protocol.as_deref().unwrap_or(""),
            event.signature.as_deref().unwrap_or(""),
        );

        Ok(xml.into_bytes())
    }

    fn content_type(&self) -> &str {
        "application/xml"
    }

    fn name(&self) -> &str {
        "iso20022"
    }
}
