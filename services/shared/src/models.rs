use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Patient {
    pub id: String,
    pub national_id: String,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: String,
    pub gender: String,
    pub blood_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabResult {
    pub id: String,
    pub patient_id: String,
    pub test_code: String,
    pub test_name: String,
    pub concept_id: String,
    pub value: f64,
    pub unit: String,
    pub reference_range: String,
    pub status: String,
    pub collected_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<TerminologyConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prescription {
    pub id: String,
    pub patient_id: String,
    pub medication_code: String,
    pub medication_name: String,
    pub concept_id: String,
    pub dosage: String,
    pub frequency: String,
    pub duration_days: u32,
    pub status: String,
    pub prescribed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<TerminologyConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminologyConcept {
    pub concept_id: String,
    pub system: String,
    pub code: String,
    pub display: String,
    pub category: String,
    pub clinical_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingAccount {
    pub patient_id: String,
    pub account_number: String,
    pub status: String,
    pub currency: String,
    pub balance: f64,
    pub credit_limit: f64,
    pub insurance_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Encounter {
    pub id: String,
    pub patient_id: String,
    pub provider_id: String,
    pub encounter_type: String,
    pub department: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockReservationItem {
    pub medication_code: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockReservation {
    pub id: String,
    pub patient_id: String,
    pub encounter_id: String,
    pub items: Vec<StockReservationItem>,
    pub status: String, // "RESERVED", "CANCELLED", "FULFILLED"
    pub reserved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceItem {
    pub description: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invoice {
    pub id: String,
    pub patient_id: String,
    pub encounter_id: String,
    pub items: Vec<InvoiceItem>,
    pub total_amount: f64,
    pub currency: String,
    pub status: String, // "PENDING", "PAID", "VOIDED"
    pub issued_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub patient_id: String,
    pub recipient: String,
    pub channel: String, // "SMS", "EMAIL", "PUSH"
    pub message: String,
    pub status: String, // "SENT", "FAILED"
    pub sent_at: String,
}

// Request and Response payloads for Gateway scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVisitRequest {
    pub patient_id: String,
    pub provider_id: String,
    pub department: String,
    pub medications: Vec<StockReservationItem>,
    pub billing_items: Vec<InvoiceItem>,
    pub notification_channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVisitResponse {
    pub encounter: Encounter,
    pub reservation: StockReservation,
    pub invoice: Invoice,
    pub notification: Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientSummaryResponse {
    pub patient: Patient,
    pub labs: Vec<LabResult>,
    pub prescriptions: Vec<Prescription>,
    pub billing: BillingAccount,
}
