use heapless::{String, Vec};
use crate::{MaxCommandLen, MaxResponseLines};

pub fn split_parameterized_resp(
  response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>,
) -> Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> {
  response_lines.reverse();

  // Handle list items
  response_lines
    .iter()
    .map(|response_line| {
      // parse response lines for parameters
      let mut v: Vec<&str, MaxResponseLines> = response_line
        .rsplit(|c: char| c == ':' || c == ',')
        .filter(|s| !s.is_empty())
        .collect();

      if v.len() > 1 {
        v.pop();
        v.reverse();
      }
      v
    })
    .collect()
}

pub fn split_parameterized_unsolicited(response_line: &str) -> (&str, Vec<&str, MaxResponseLines>) {
  let mut parameters: Vec<&str, MaxResponseLines> = response_line
      .rsplit(|c: char| c == ':' || c == ',')
      .filter(|s| !s.is_empty())
      .collect();

    let cmd = parameters.pop().unwrap();
    parameters.reverse();
    (cmd, parameters)
}
