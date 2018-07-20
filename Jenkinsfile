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
        node {
            checkout scm
            dir('ci') {
                def image = docker.build('collectd-rust-image', "--build-arg UBUNTU_VERSION=${os} --build-arg COLLECTD_VERSION=${collectd} .")
                image.inside("-v ${WORKSPACE}:/tmp -e CARGO_HOME=/tmp/.cargo") {
                    checkout scm
                    sh "COLLECTD_VERSION=${collectd} ci/test.sh"
                    junit 'TestResults-*'
                }
            }
        }
    }
}
