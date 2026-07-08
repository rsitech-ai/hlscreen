use hls_core::{HlsError, HlsResult, market_state::FeatureSnapshot};

use crate::{
    dsl::parser::{parse_filter, parse_sort},
    presets::find_preset,
    row::ScreenRow,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScreenRequest {
    pub preset: Option<String>,
    pub where_expr: Option<String>,
    pub sort: Option<String>,
}

impl ScreenRequest {
    pub fn preset(name: impl Into<String>) -> Self {
        Self {
            preset: Some(name.into()),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.preset.is_none() && self.where_expr.is_none() && self.sort.is_none()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScreenEngine;

impl ScreenEngine {
    pub fn apply(
        &self,
        rows: &[FeatureSnapshot],
        request: &ScreenRequest,
    ) -> HlsResult<Vec<FeatureSnapshot>> {
        let (where_expr, sort) = resolved_rule(request)?;
        let filter = where_expr.as_deref().map(parse_filter).transpose()?;
        let sort = sort.as_deref().map(parse_sort).transpose()?;

        let mut visible = Vec::new();
        for row in rows {
            let matches = filter
                .as_ref()
                .map(|filter| filter.matches(ScreenRow::new(row)))
                .transpose()?
                .unwrap_or(true);
            if matches {
                visible.push(row.clone());
            }
        }

        if let Some(sort) = sort {
            visible
                .sort_by(|left, right| sort.compare(ScreenRow::new(left), ScreenRow::new(right)));
        }

        Ok(visible)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScreenSession {
    engine: ScreenEngine,
    active_rows: Vec<FeatureSnapshot>,
    active_request: Option<ScreenRequest>,
}

impl ScreenSession {
    pub fn apply(
        &mut self,
        rows: &[FeatureSnapshot],
        request: &ScreenRequest,
    ) -> HlsResult<&[FeatureSnapshot]> {
        let visible = self.engine.apply(rows, request)?;
        self.active_request = Some(request.clone());
        self.active_rows = visible;
        Ok(&self.active_rows)
    }

    pub fn active_rows(&self) -> &[FeatureSnapshot] {
        &self.active_rows
    }

    pub fn active_request(&self) -> Option<&ScreenRequest> {
        self.active_request.as_ref()
    }
}

fn resolved_rule(request: &ScreenRequest) -> HlsResult<(Option<String>, Option<String>)> {
    let mut where_expr = None;
    let mut sort = None;

    if let Some(name) = &request.preset {
        let Some(preset) = find_preset(name) else {
            return Err(HlsError::Config(format!("unknown preset '{name}'")));
        };
        where_expr = Some(preset.where_expr.to_owned());
        sort = Some(preset.sort.to_owned());
    }

    if let Some(custom_where) = &request.where_expr {
        where_expr = Some(custom_where.clone());
    }
    if let Some(custom_sort) = &request.sort {
        sort = Some(custom_sort.clone());
    }

    Ok((where_expr, sort))
}
