def runs = ['14.04':'5.4',
            '16.04':'5.5',
            '17.10':'5.7']

def steps = runs.collectEntries {
    ["ubuntu $it.key": job(it.key, it.value)]
}

properties([
    pipelineTriggers([
        cron('@weekly')
    ])
])

parallel steps

def job(os, collectd) {
    return {
        docker.image("ubuntu:${os}").inside {
            checkout scm
            sh "COLLECTD_VERSION=${collectd} ci/full.sh"
            junit 'TestResults-*'
        }
    }
}
