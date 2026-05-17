// Temporary mock data for download-folders responsive testing
import { MailboxData, MailboxListResponse } from './api';

let _id = 1;
const m = (overrides: Partial<MailboxData>): MailboxData => ({
  account_id: 1,
  attributes: [],
  delimiter: '/',
  exists: Math.floor(Math.random() * 5000) + 100,
  id: _id++,
  name: '',
  uid_next: null,
  uid_validity: null,
  unseen: null,
  ...overrides,
});

const SHORT = [
  m({ name: 'INBOX' }),
  m({ name: 'INBOX/Drafts' }),
  m({ name: 'INBOX/Sent' }),
  m({ name: 'INBOX/Trash' }),
  m({ name: 'INBOX/Archive' }),
  m({ name: 'INBOX/Spam' }),
  m({ name: 'INBOX/Templates' }),
];

const PROJECTS = [
  m({ name: 'INBOX/Projects' }),
  m({ name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review' }),
  m({ name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Drafts' }),
  m({
    name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Drafts/Revision 3 - Updated Forecast Models and Department Sign-off Required',
    attributes: [{ attr: 'HasChildren', extension: null }],
  }),
  m({
    name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Drafts/Revision 3 - Updated Forecast Models and Department Sign-off Required/Comments from CFO',
  }),
  m({
    name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Drafts/Revision 3 - Updated Forecast Models and Department Sign-off Required/Attachments',
  }),
  m({ name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Final' }),
  m({ name: 'INBOX/Projects/Q4 2025 Financial Reports and Annual Budget Planning Review/Final/Approved with Amendments' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Kickoff' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Kickoff/Meeting Minutes and Action Items' }),
  m({
    name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Vendor Proposals',
    attributes: [{ attr: 'HasNoChildren', extension: null }],
  }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Vendor Proposals/AWS Proposal Package' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Vendor Proposals/Azure Proposal Package' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Vendor Proposals/GCP Proposal Package' }),
  m({ name: 'INBOX/Projects/Q1 2026 Strategic Initiative - Cloud Infrastructure Migration Assessment and Vendor Selection/Internal Review and Scoring Committee' }),
  m({ name: 'INBOX/Projects/HR Portal Redesign - Employee Self-Service Platform Modernization' }),
  m({ name: 'INBOX/Projects/HR Portal Redesign - Employee Self-Service Platform Modernization/Wireframes and Mockups' }),
  m({ name: 'INBOX/Projects/HR Portal Redesign - Employee Self-Service Platform Modernization/Wireframes and Mockups/Iteration 1' }),
  m({ name: 'INBOX/Projects/HR Portal Redesign - Employee Self-Service Platform Modernization/Wireframes and Mockups/Iteration 2' }),
  m({ name: 'INBOX/Projects/HR Portal Redesign - Employee Self-Service Platform Modernization/Usability Testing Results and Feedback Compilation' }),
];

const CLIENTS = [
  m({ name: 'INBOX/Clients' }),
  m({
    name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026',
    attributes: [{ attr: 'HasChildren', extension: null }],
  }),
  m({ name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Contract Documents' }),
  m({ name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Contract Documents/Redlined Versions' }),
  m({
    name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Contract Documents/Redlined Versions/Legal Review Round 1',
  }),
  m({
    name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Contract Documents/Redlined Versions/Legal Review Round 2 - Final',
  }),
  m({
    name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Invoices and Payment Records',
  }),
  m({ name: 'INBOX/Clients/Acme Corporation - Enterprise Software Licensing and Support Agreement Renewal 2026/Support Tickets and Correspondence' }),
  m({
    name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement',
    attributes: [{ attr: 'HasChildren', extension: null }],
  }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 1 Discovery and Assessment' }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 1 Discovery and Assessment/Stakeholder Interviews' }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 1 Discovery and Assessment/Current State Architecture Documentation' }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 2 Implementation Roadmap' }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 2 Implementation Roadmap/Sprint Planning and Resource Allocation' }),
  m({ name: 'INBOX/Clients/Globex Industries - Multi-Year Digital Transformation Consulting Engagement/Phase 2 Implementation Roadmap/Risk Assessment and Mitigation Strategies' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026/Penetration Testing Reports' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026/Penetration Testing Reports/External Network Assessment' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026/Penetration Testing Reports/Internal Network Assessment' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026/Penetration Testing Reports/Web Application Security Scan Results' }),
  m({ name: 'INBOX/Clients/Initech Solutions - Cybersecurity Audit and Compliance Remediation Program 2026/Compliance Gap Analysis and Remediation Tracking' }),
  m({ name: 'INBOX/Clients/Massive Dynamic - Research Collaboration on Advanced Machine Learning Applications in Healthcare Informatics' }),
  m({ name: 'INBOX/Clients/Massive Dynamic - Research Collaboration on Advanced Machine Learning Applications in Healthcare Informatics/Data Sharing Agreements and Ethics Board Approvals' }),
  m({ name: 'INBOX/Clients/Massive Dynamic - Research Collaboration on Advanced Machine Learning Applications in Healthcare Informatics/Literature Review and Prior Art Analysis' }),
  m({ name: 'INBOX/Clients/Massive Dynamic - Research Collaboration on Advanced Machine Learning Applications in Healthcare Informatics/Model Training Datasets and Validation Results' }),
];

const NOTIFICATIONS = [
  m({ name: 'INBOX/Notifications' }),
  m({ name: 'INBOX/Notifications/GitHub Enterprise - Pull Request Reviews and CI/CD Pipeline Status Updates' }),
  m({ name: 'INBOX/Notifications/Jira Service Management - Incident Response Alerts and Escalation Notifications' }),
  m({ name: 'INBOX/Notifications/Confluence - Documentation Updates and Page Modification Summaries' }),
  m({ name: 'INBOX/Notifications/Slack Workspace - Channel Highlights and Direct Message Digest Compilation' }),
  m({ name: 'INBOX/Notifications/Datadog - Application Performance Monitoring Alerts and Anomaly Detection Reports' }),
  m({ name: 'INBOX/Notifications/PagerDuty - On-Call Rotation Schedule and Incident Acknowledgment Confirmations' }),
  m({ name: 'INBOX/Notifications/Microsoft 365 - Calendar Invitations and Meeting Room Booking Confirmations' }),
];

const NEWSLETTERS = [
  m({ name: 'INBOX/Newsletters' }),
  m({ name: 'INBOX/Newsletters/Rust Weekly - Community Updates, Crate Highlights, and RFC Progress Tracking Digest' }),
  m({
    name: 'INBOX/Newsletters/Systems Programming Insider - Deep Dive Articles on Memory Management and Concurrency Patterns',
    attributes: [{ attr: 'HasNoChildren', extension: null }],
  }),
  m({ name: 'INBOX/Newsletters/Cloud Native Computing Foundation - Kubernetes Ecosystem Updates and Project Maturity Reports' }),
  m({ name: 'INBOX/Newsletters/Software Architecture Monthly - Case Studies in Distributed Systems Design and Microservices Patterns' }),
  m({ name: 'INBOX/Newsletters/DevOps Weekly Digest - Tool Reviews, Pipeline Optimization Techniques, and Platform Engineering Insights' }),
  m({ name: 'INBOX/Newsletters/Information Security Briefing - CVE Disclosures, Threat Intelligence Reports, and Zero-Day Advisories' }),
  m({ name: 'INBOX/Newsletters/Tech Leadership Forum - Engineering Management Best Practices and Organizational Scaling Strategies' }),
];

export const MOCK_MAILBOX_LIST: MailboxListResponse = {
  status: 'ready',
  mailboxes: [
    ...SHORT,
    ...PROJECTS,
    ...CLIENTS,
    ...NOTIFICATIONS,
    ...NEWSLETTERS,
  ],
};
