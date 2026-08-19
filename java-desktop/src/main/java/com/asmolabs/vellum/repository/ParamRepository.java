package com.asmolabs.vellum.repository;

import com.asmolabs.vellum.domain.entity.Param;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;

@Repository
public interface ParamRepository extends JpaRepository<Param, String> {
    List<Param> findByPluginId(String pluginId);
    Optional<Param> findByPluginIdAndKey(String pluginId, String key);
}
