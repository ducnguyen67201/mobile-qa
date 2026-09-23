/* Narrow setuid transition: move only this process, then permanently drop root.
 * The parent retains the same PID/process group for its existing cleanup proof.
 * No caller-selected executable, path, PID, command, or environment is used while
 * privileged. The privileged files and SDK are installed root-owned by the AMI.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <grp.h>
#include <pwd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define EMULATOR "/opt/mobile-qa/android-sdk/emulator/emulator"
#define READY "/run/mobile-qa-network/policy.ready"
#define GROUP "/sys/fs/cgroup/mobile-qa-emulators/cgroup.procs"

static void fail(void) {
    static const char message[] = "mobile_qa_emulator_isolation_unavailable\n";
    (void)!write(STDERR_FILENO, message, sizeof(message) - 1);
    _exit(126);
}

static int trusted_file(const char *path, int flags) {
    struct stat info;
    int fd = open(path, flags | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0 || fstat(fd, &info) || !S_ISREG(info.st_mode) ||
        info.st_uid != 0 || (info.st_mode & 0022) != 0) fail();
    return fd;
}

static void trusted_directory(const char *path) {
    struct stat info;
    int fd = open(path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0 || fstat(fd, &info) || info.st_uid != 0 ||
        (info.st_mode & 0022) != 0) fail();
    if (close(fd)) fail();
}

static void current_policy(void) {
    char ready[128], boot[64];
    int policy = trusted_file(READY, O_RDONLY);
    ssize_t length = read(policy, ready, sizeof(ready));
    if (close(policy) || length != 102) fail();
    int source = open("/proc/sys/kernel/random/boot_id", O_RDONLY | O_CLOEXEC);
    if (source < 0 || read(source, boot, sizeof(boot)) != 37 || close(source)) fail();
    if (memcmp(boot, ready, 37) != 0 || ready[101] != '\n') fail();
    for (size_t i = 37; i < 101; ++i)
        if (!((ready[i] >= '0' && ready[i] <= '9') ||
              (ready[i] >= 'a' && ready[i] <= 'f'))) fail();
}

static char *state_path(const char *name) {
    const char *value = getenv(name);
    static const char prefix[] = "/var/lib/mobile-qa/";
    if (!value || strlen(value) > 1024 ||
        strncmp(value, prefix, sizeof(prefix) - 1) != 0 ||
        strstr(value, "/../") || strstr(value, "/./") ||
        strstr(value, "//") || strchr(value, '\n') || strchr(value, '\r')) fail();
    size_t length = strlen(value);
    if (length < sizeof(prefix) || value[length - 1] == '.' || value[length - 1] == '/') fail();
    char *copy = strdup(value);
    if (!copy) fail();
    return copy;
}

static void environment(const char *avd, const char *user) {
    if (clearenv() ||
        setenv("PATH", "/usr/local/bin:/usr/bin:/bin", 1) ||
        setenv("HOME", "/var/lib/mobile-qa", 1) ||
        setenv("LANG", "C.UTF-8", 1) || setenv("LC_ALL", "C.UTF-8", 1) ||
        setenv("TZ", "UTC", 1) ||
        setenv("JAVA_HOME", "/usr/lib/jvm/java-17-openjdk-amd64", 1) ||
        setenv("ANDROID_HOME", "/opt/mobile-qa/android-sdk", 1) ||
        setenv("ANDROID_AVD_HOME", avd, 1) || setenv("ANDROID_USER_HOME", user, 1)) fail();
}

int main(int argc, char **argv) {
    if (argc < 2 || argc > 64 || strcmp(argv[1], EMULATOR) != 0 || geteuid() != 0) fail();
    size_t total = 0;
    for (int i = 1; i < argc; ++i) {
        size_t length = strnlen(argv[i], 4097);
        if (length > 4096 || (total += length) > 16384) fail();
    }
    struct passwd *account = getpwnam("mobile-qa");
    if (!account || account->pw_uid == 0 || getuid() != account->pw_uid) fail();
    uid_t owner = account->pw_uid;
    gid_t group = account->pw_gid;
    struct group *kvm = getgrnam("kvm");
    if (!kvm) fail();
    gid_t device_group = kvm->gr_gid;
    char *avd = state_path("ANDROID_AVD_HOME");
    char *user = state_path("ANDROID_USER_HOME");

    trusted_directory("/opt");
    trusted_directory("/opt/mobile-qa");
    trusted_directory("/opt/mobile-qa/android-sdk");
    trusted_directory("/opt/mobile-qa/android-sdk/emulator");
    trusted_directory("/run/mobile-qa-network");
    trusted_directory("/sys/fs/cgroup/mobile-qa-emulators");
    int executable = trusted_file(EMULATOR, O_RDONLY);
    if (close(executable)) fail();
    current_policy();

    int destination = trusted_file(GROUP, O_WRONLY);
    char process[32];
    int count = snprintf(process, sizeof(process), "%ld\n", (long)getpid());
    if (count < 1 || (size_t)count >= sizeof(process) ||
        write(destination, process, (size_t)count) != count || close(destination)) fail();

    /* No environment-controlled loader or executable can observe elevated IDs. */
    environment(avd, user);
    free(avd);
    free(user);
    if (setgroups(1, &device_group) || setresgid(group, group, group) ||
        setresuid(owner, owner, owner) || getuid() != owner || geteuid() != owner ||
        getgid() != group || getegid() != group ||
        prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)) fail();
    execv(EMULATOR, &argv[1]);
    fail();
}
