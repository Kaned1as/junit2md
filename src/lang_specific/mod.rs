/// Removes Java package from a fully-qualified class name.
/// If class name doesn't contain package, does nothing.
/// 
/// Arguments:
/// * `name` - class name.
pub(super) fn omit_java_package(name: &str) -> &str {
    if !name.contains('.') || name.contains(' ') {
        // not a java class name
        return name;
    }

    let last_dot_idx = name.rfind('.').unwrap() + 1;
    if name.len() == last_dot_idx {
        // string was ending with dot? what is that?
        return name;
    }

    return name.get(last_dot_idx..name.len()).unwrap_or(name);
}