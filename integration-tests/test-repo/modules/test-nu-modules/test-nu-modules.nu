#!/usr/libexec/bluebuild/nu/nu

def main [$arg] {
    # Parse the JSON string into a NuShell table
    let parsed_json = ($arg | from json)

    # List all top-level properties and their values
    print "Top-level properties and values:"
    $parsed_json | items {|key, value| $"Property: ($key), Value: ($value)" }
}
