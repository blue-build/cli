#!/usr/bin/env nu

# Runs an eval using the string passed in.
def check_eval []: string -> bool {
    let eval = $in
    let check = /bin/sh -c $'($eval)' | complete

    if $check.exit_code != 0 {
        print -e $"(ansi yellow)Check failed for (ansi cyan)'($eval)'(ansi reset)"
        false
    } else {
        print -e $"(ansi green)Check passed for (ansi cyan)'($eval)'(ansi reset)"
        true
    }
}

# Gets the os-release values and
# stores them in a parsed table.
def get_os_release []: nothing -> record {
    open --raw /etc/os-release
        | lines
        | parse '{key}={value}'
        | transpose --ignore-titles -dr
        | str trim -c '"'
        | str trim -c "'"
}

# Checks that values specified match os-release values.
def check_os_release []: record -> bool {
    print -e $'(ansi yellow)Performing os-release check(ansi reset)'
    let os_release = $in

    let os_vars = get_os_release

    for $var in ($os_release | columns) {
        print -e $'(ansi yellow)Checking os-release var (ansi cyan)($var)(ansi reset)'

        let $os_value = $os_vars
            | default '' $var
            | get $var
        let check_value = $os_release | get $var

        match ($check_value | describe) {
            'string' => {
                if $check_value != $os_value {
                    print -e $'(ansi red)OS release var (ansi cyan)($var)=($os_value)(ansi red) did not equal (ansi yellow)($check_value)(ansi reset)'
                    return false
                }
            }
            $type if ($type | str starts-with 'list') => {
                let any = $check_value
                    | any { $in == $os_value }

                if not $any {
                    print -e $'(ansi red)OS release var (ansi cyan)($var)=($os_value)(ansi red) did not equal any of (ansi yellow)($check_value)(ansi reset)'
                    return false
                }
            }
            _ => {
                return false
            }
        }
    }

    true
}

# Checks that no values specified match os-release values.
def check_not_os_release []: record -> bool {
    print -e $'(ansi yellow)Performing negated os-release check(ansi reset)'

    let not_os_release = $in

    let os_vars = get_os_release

    for $var in ($not_os_release | columns) {
        print -e $'(ansi yellow)Checking os-release var (ansi cyan)($var)(ansi reset)'

        let os_value = $os_vars
            | default '' $var
            | get $var
        let check_value = $not_os_release | get $var

        match ($check_value | describe) {
            'string' => {
                if $check_value == $os_value {
                    print -e $'(ansi red)OS release var (ansi cyan)($var)=($os_value)(ansi red) equaled (ansi yellow)($check_value)(ansi reset)'
                    return false
                }
            }
            $type if ($type | str starts-with 'list') => {
                let all = $check_value
                    | all { $in != $os_value }

                if not $all {
                    print -e $'(ansi red)OS release var (ansi cyan)($var)=($os_value)(ansi red) equaled one of (ansi yellow)($check_value)(ansi reset)'
                    return false
                }
            }
            _ => {
                return false
            }
        }
    }

    true
}

def check_env []: record -> bool {
    let check_env = $in
        | default null exists
        | default null not-exists
        | default {} equals
        | default {} not-equals

    let exists = if $check_env.exists == null {
        true
    } else {
        match ($check_env.exists | describe) {
            'string' => {
                $check_env.exists in $env
            }
            'list<string>' => {
                $check_env.exists
                    | all {
                        let check = $in in $env

                        if not $check {
                            print -e $'(ansi red)Environment variable (ansi cyan)($in)(ansi red) does not exist.(ansi reset)'
                        }

                        $check
                    }
            }
            _ => {
                false
            }
        }
    }

    let not_exists = if $check_env.not-exists == null {
        true
    } else {
        match ($check_env.not-exists | describe) {
            'string' => {
                not ($check_env.not-exists in $env)
            }
            'list<string>' => {
                not ($check_env.not-exists
                    | any {
                        let check = $in in $env

                        if $check {
                            print -e $'(ansi red)Environment variable (ansi cyan)($in)(ansi red) exists.(ansi reset)'
                        }

                        $check
                    })
            }
            _ => {
                false
            }
        }
    }

    let equals = $check_env.equals
        | columns
        | all {|var_name|
            let value = $check_env.equals
                | get $var_name

            match ($value | describe) {
                'string' => {
                    let actual = $env
                        | default null $var_name
                        | get $var_name
                    let check = $var_name in $env and $actual == $value

                    if not $check {
                        print -e $'(ansi red)Environment variable (ansi cyan)($var_name)(ansi red) was (ansi yellow)($actual)(ansi red) but expected (ansi green)($value)(ansi reset)'
                    }

                    $check
                }
                'list<string>' => {
                    let actual = $env
                        | default null $var_name
                        | get $var_name
                    let check = $value
                        | any {|value|
                            $var_name in $env and $actual == $value
                        }

                    if not $check {
                        print -e $'(ansi red)Environment variable (ansi cyan)($var_name)(ansi red) was (ansi yellow)($actual)(ansi red) but expected one of (ansi green)($value)(ansi reset)'
                    }

                    $check
                }
                _ => {
                    false
                }
            }
        }

    let not_equals = $check_env.not-equals
        | columns
        | all {|var_name|
            let value = $check_env.not-equals
                | get $var_name

            match ($value | describe) {
                'string' => {
                    let actual = $env
                        | default null $var_name
                        | get $var_name
                    let check = not ($var_name in $env) or $actual != $value

                    if not $check {
                        print -e $"(ansi red)Environment variable (ansi cyan)($var_name)(ansi red) was not expected to be (ansi yellow)($actual)(ansi reset)"
                    }

                    $check
                }
                'list<string>' => {
                    let actual = $env
                        | default null $var_name
                        | get $var_name
                    let check = $value
                        | all {|value|
                            not ($var_name in $env) or $actual != $value
                        }

                    if not $check {
                        print -e $"(ansi red)Environment variable (ansi cyan)($var_name)(ansi red) was (ansi yellow)($actual)(ansi red) but wasn't expected to be any of (ansi green)($value)(ansi reset)"
                    }

                    $check
                }
                _ => {
                    false
                }
            }
        }

    $exists and $not_exists and $equals and $not_equals
}

def main [config: string]: nothing -> nothing {
    let config = $config
        | from json
        | default null if
        | default 'module' type

    if $config.if != null {
        print -e $"(ansi yellow)Checking if '(ansi cyan)($config.type)(ansi yellow)' should run(ansi reset)"

        let if_type = $config.if? | describe

        let should_run = match $if_type {
            $type if ($type | str starts-with 'record') => {
                let mod_if = $config
                    | get if

                let eval = if $mod_if.eval? != null {
                    $mod_if.eval | check_eval
                } else {
                    true
                }
                let os_release = if $mod_if.os-release? != null {
                    $mod_if.os-release | check_os_release
                } else {
                    true
                }
                let not_os_release = if $mod_if.not-os-release? != null {
                    $mod_if.not-os-release | check_not_os_release
                } else {
                    true
                }
                let check_env = if $mod_if.env? != null {
                    $mod_if.env | check_env
                } else {
                    true
                }

                $eval and $os_release and $not_os_release and $check_env
            }
            'string' => {
                $config.if | check_eval
            }
            $type => {
                print -e $'(ansi red)Unrecognized type (ansi yellow)($type)(ansi reset)'
                false
            }
        }

        if $should_run {
            print -e $"(ansi green)Continuing with '(ansi cyan)($config.type)(ansi green)' execution(ansi reset)"
            exit 0
        } else {
            print -e $"(ansi red)Not continuing with '(ansi cyan)($config.type)(ansi red)' execution(ansi reset)"
            exit 2
        }
    }
}
