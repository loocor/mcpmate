use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use iri_string::{
    spec::UriSpec,
    template::{
        UriTemplateStr,
        simple_context::{SimpleContext, Value as TemplateValue},
    },
};
use serde_json::{Map, Value};
use url::Url;

pub(crate) fn expand_resource_template(
    uri_template: &str,
    arguments: Option<&Map<String, Value>>,
) -> Result<String> {
    let template =
        UriTemplateStr::new(uri_template).with_context(|| format!("Invalid RFC 6570 URI template '{uri_template}'"))?;
    let declared = template
        .variables()
        .map(|variable| variable.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let mut context = SimpleContext::new();

    for (name, value) in arguments.into_iter().flatten() {
        if !declared.contains(name) {
            bail!("Argument '{name}' is not declared by URI template '{uri_template}'");
        }
        if value.is_null() {
            continue;
        }
        context.insert(name, template_value(name, value)?);
    }

    let expanded = template
        .expand::<UriSpec, _>(&context)
        .context("Failed to expand RFC 6570 URI template")?
        .to_string();
    Url::parse(&expanded).with_context(|| format!("Expanded URI '{expanded}' is invalid"))?;
    Ok(expanded)
}

fn template_value(
    name: &str,
    value: &Value,
) -> Result<TemplateValue> {
    match value {
        Value::String(value) => Ok(TemplateValue::String(value.clone())),
        Value::Number(value) => Ok(TemplateValue::String(value.to_string())),
        Value::Bool(value) => Ok(TemplateValue::String(value.to_string())),
        Value::Array(values) => values
            .iter()
            .map(|value| scalar_value(name, value))
            .collect::<Result<Vec<_>>>()
            .map(TemplateValue::List),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), scalar_value(name, value)?)))
            .collect::<Result<Vec<_>>>()
            .map(TemplateValue::Assoc),
        Value::Null => unreachable!("top-level JSON null is handled as an undefined template variable"),
    }
}

fn scalar_value(
    argument_name: &str,
    value: &Value,
) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Array(_) | Value::Object(_) | Value::Null => {
            bail!("Argument '{argument_name}' collections may contain only string, number, or boolean values")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::expand_resource_template;

    #[test]
    fn expands_a_scalar_uri_template() {
        let arguments = json!({ "resourceId": 42 });
        assert_eq!(
            expand_resource_template("test://dynamic/{resourceId}", arguments.as_object(),).expect("expand template"),
            "test://dynamic/42"
        );
    }

    #[test]
    fn follows_rfc6570_reserved_prefix_and_percent_encoding_rules() {
        let arguments = json!({
            "path": "docs/a b",
            "term": "abcdef",
            "query": "space / slash",
            "plain": "a/b c",
        });
        assert_eq!(
            expand_resource_template(
                "https://example.test/{+path}/search/{term:3}{?query,plain}",
                arguments.as_object(),
            )
            .expect("expand template"),
            "https://example.test/docs/a%20b/search/abc?query=space%20%2F%20slash&plain=a%2Fb%20c"
        );
    }

    #[test]
    fn expands_scalar_list_and_associative_values_with_explode() {
        let arguments = json!({
            "scalar": true,
            "list": ["red", 2, false],
            "map": { "dot": ".", "semi": ";" },
        });
        assert_eq!(
            expand_resource_template("https://example.test{?scalar,list*,map*}", arguments.as_object(),)
                .expect("expand template"),
            "https://example.test?scalar=true&list=red&list=2&list=false&dot=.&semi=%3B"
        );
    }

    #[test]
    fn omits_missing_and_null_arguments() {
        let arguments = json!({ "optional": null });
        assert_eq!(
            expand_resource_template("https://example.test/items{?optional,missing}", arguments.as_object())
                .expect("expand template"),
            "https://example.test/items"
        );
    }

    #[test]
    fn rejects_undeclared_and_nested_arguments() {
        let undeclared = json!({ "other": "value" });
        assert!(expand_resource_template("https://example.test/{id}", undeclared.as_object()).is_err());

        let nested_list = json!({ "id": [["nested"]] });
        assert!(expand_resource_template("https://example.test/{id}", nested_list.as_object()).is_err());

        let nested_map = json!({ "id": { "nested": { "value": 1 } } });
        assert!(expand_resource_template("https://example.test/{id}", nested_map.as_object()).is_err());
    }

    #[test]
    fn rejects_an_expansion_that_is_not_an_absolute_uri() {
        let arguments = json!({ "id": "resource" });
        assert!(expand_resource_template("relative/{id}", arguments.as_object()).is_err());
    }
}
