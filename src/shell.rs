pub fn init() -> &'static str {
    r#"gwt() {
  if [ "$#" -eq 1 ] && [ "$1" = "list" ] && [ -t 0 ] && [ -t 1 ]; then
    local _gwt_path
    _gwt_path="$(command gwt list --select)" || return $?
    if [ -n "$_gwt_path" ]; then
      builtin cd -- "$_gwt_path"
    fi
  else
    command gwt "$@"
  fi
}"#
}
