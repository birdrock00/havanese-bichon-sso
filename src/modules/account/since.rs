//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
    modules::error::{code::ErrorCode, BichonResult},
    raise_error,
};
use chrono::{Datelike, Days, Local, Months, NaiveDate, Utc};
use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct DateSince {
    /// Absolute date boundary in ISO 8601 format (YYYY-MM-DD)
    ///
    /// ### Validation Rules
    /// - Must match exact format `^\d{4}-\d{2}-\d{2}$`
    /// - Date must be logically valid (e.g. no 2025-05-01)
    ///
    /// ### Example
    /// ```json
    /// {
    ///   "fixed": "2025-05-01"
    /// }
    /// ```
    #[oai(validator(pattern = r"^\d{4}-\d{2}-\d{2}$"))]
    pub fixed: Option<String>,
    /// Relative time period from current date
    ///
    /// ### Constraints
    /// - Value must be ≥ 1
    /// - Units support day/month/year granularity
    ///
    /// ### Example
    /// ```json
    /// {
    ///   "relative": {
    ///     "unit": "Days",
    ///     "value": 7
    ///   }
    /// }
    /// ```
    pub relative: Option<RelativeDate>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Enum)]
pub enum Unit {
    #[default]
    Days,
    Months,
    Years,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct RelativeDate {
    /// The time unit to use for the offset (days, months, or years)
    pub unit: Unit,
    /// The quantity of time units to offset (must be a positive integer)
    #[oai(validator(minimum(value = "1")))]
    pub value: u32,
}

impl RelativeDate {
    pub fn validate_date(&self) -> BichonResult<()> {
        if self.value == 0 {
            return Err(raise_error!(
                "Value must be greater than 0".into(),
                ErrorCode::InvalidParameter
            ));
        }

        let now = Local::now();
        let date = match self.unit {
            Unit::Days => now.checked_sub_days(Days::new(self.value as u64)),
            Unit::Months => now.checked_sub_months(Months::new(self.value)),
            Unit::Years => now.checked_sub_months(Months::new(self.value * 12)),
        };

        let date = date.ok_or_else(|| {
            raise_error!(
                "Invalid date: the calculated date is earlier than 1970 or an overflow occurred."
                    .into(),
                ErrorCode::InvalidParameter
            )
        })?;

        let naive_date = date.date_naive();

        // Check if the date is before 1970
        if naive_date.year() < 1970 {
            return Err(raise_error!(
                format!(
                    "Date cannot be earlier than 1970-01-01. Provided: '{}'",
                    naive_date
                ),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }

    fn compute_date(&self) -> BichonResult<chrono::DateTime<Local>> {
        if self.value == 0 {
            return Err(raise_error!(
                "Value must be greater than 0".into(),
                ErrorCode::InvalidParameter
            ));
        }

        let now = Local::now();
        let date = match self.unit {
            Unit::Days => now.checked_sub_days(Days::new(self.value as u64)),
            Unit::Months => now.checked_sub_months(Months::new(self.value)),
            Unit::Years => now.checked_sub_months(Months::new(self.value * 12)),
        };

        let date = date.ok_or_else(|| {
            raise_error!(
                "Invalid date: the calculated date is earlier than 1970 or an overflow occurred."
                    .into(),
                ErrorCode::InvalidParameter
            )
        })?;

        let naive_date = date.date_naive();
        if naive_date.year() < 1970 {
            return Err(raise_error!(
                format!(
                    "Date cannot be earlier than 1970-01-01. Provided: '{}'",
                    naive_date
                ),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(date)
    }

    pub fn calculate_date(&self) -> BichonResult<String> {
        let date = self.compute_date()?;
        Ok(date.format("%d-%b-%Y").to_string())
    }
}

impl DateSince {
    pub fn validate(&self) -> BichonResult<()> {
        match (&self.fixed, &self.relative) {
            // If only `relative` is provided
            (None, Some(r)) => {
                r.validate_date()?;
            }
            // If only `fixed` is provided
            (Some(fixed), None) => {
                self.validate_fixed_date(fixed)?;
            }
            // If both or neither are provided
            _ => {
                return Err(raise_error!(
                    "Invalid input: You must provide either 'fixed' or 'relative', but not both."
                        .to_string(),
                    ErrorCode::InvalidParameter
                ));
            }
        }
        Ok(())
    }

    fn validate_fixed_date(&self, fixed: &str) -> BichonResult<()> {
        // Try to parse the input string as YYYY-MM-DD
        let date = NaiveDate::parse_from_str(fixed, "%Y-%m-%d").map_err(|_| {
            raise_error!(
                format!(
                "Invalid date format. Expected 'YYYY-MM-DD'. Example: '2024-11-19'. Provided: '{}'",
                fixed
            ),
                ErrorCode::InvalidParameter
            )
        })?;

        let now = Utc::now().date_naive();

        // Check if the date is in the future
        if date >= now {
            return Err(raise_error!(
                format!(
                    "Date cannot be in the future. Provided: '{}', Today: '{}'",
                    fixed,
                    now.format("%Y-%m-%d")
                ),
                ErrorCode::InvalidParameter
            ));
        }

        // Check if the date is before 1970
        if date.year() < 1970 {
            return Err(raise_error!(
                format!(
                    "Date cannot be earlier than 1970-01-01. Provided: '{}'",
                    fixed
                ),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }

    fn format_user_date(&self, fixed: &str) -> BichonResult<String> {
        let date = NaiveDate::parse_from_str(fixed, "%Y-%m-%d").map_err(|_| {
            raise_error!(
                format!(
                "Invalid date format. Expected 'YYYY-MM-DD'. Example: '2024-11-19'. Provided: '{}'",
                fixed
            ),
                ErrorCode::InvalidParameter
            )
        })?;
        // Format the date into "%d-%b-%Y" format
        Ok(date.format("%d-%b-%Y").to_string())
    }

    pub fn since_date(&self) -> BichonResult<String> {
        // Handle the case where only one of `fixed` or `relative` is provided
        if let Some(r) = &self.relative {
            // If `relative` is provided, calculate the date
            r.calculate_date()
        } else if let Some(f) = &self.fixed {
            // If `fixed` is provided, format the date
            self.format_user_date(f)
        } else {
            // If neither is provided, return an error
            Err(raise_error!(
                "You must provide either a 'fixed' or 'relative' date.".to_string(),
                ErrorCode::InvalidParameter
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::modules::account::since::{DateSince, RelativeDate, Unit};

    #[test]
    fn test1() {
        let e = DateSince {
            fixed: Some("2014-09-12".to_string()),
            relative: None,
        };

        e.validate().unwrap();

        println!("{}", e.since_date().unwrap());

        let e = DateSince {
            fixed: None,
            relative: Some(RelativeDate {
                unit: Unit::Days,
                value: 1,
            }),
        };

        e.validate().unwrap();

        println!("{}", e.since_date().unwrap());
    }
}
