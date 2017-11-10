def runs = ['14.04':'collectd-54',
            '16.04':'collectd-55',
            '17.04':'collectd-57']

def steps = runs.collectEntries {
    ['ubuntu $it.key': job(it.key, it.value)]
}

parallel steps

def job(os, collectd) {
    return {
        docker.image("ubuntu:${os}").inside {
            checkout scm
            sh 'ci/setup.sh'
            if (!os.equals("17.04")) {
                sh 'wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -'
                sh 'apt-get install -y llvm-3.9-dev libclang-3.9-dev clang-3.9'
            }

            if (os.equals("14.04")) {
                sh 'cp -r /usr/include/collectd/liboconfig /usr/include/collectd/core/.'
            }

            sh "VERSION=${collectd} ci/test.sh"
            junit 'TestResults.xml'
        }
    }
}
